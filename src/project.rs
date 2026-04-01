use crate::fragment::blob_oid_bytes;
use crate::manifest::Manifest;
use std::collections::BTreeMap;
use std::path::Path;

/// Result of projecting: target path → (content bytes, blob OID).
#[derive(Debug)]
pub struct Projection {
    pub files: BTreeMap<String, ProjectedFile>,
}

#[derive(Debug)]
pub struct ProjectedFile {
    pub content: Vec<u8>,
    pub oid: String,
}

/// Error during projection.
#[derive(Debug)]
pub enum ProjectError {
    /// Source file not found.
    NotFound(String),
    /// IO error reading source.
    Io(std::io::Error),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::NotFound(path) => write!(f, "source not found: {}", path),
            ProjectError::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl From<std::io::Error> for ProjectError {
    fn from(e: std::io::Error) -> Self {
        ProjectError::Io(e)
    }
}

/// Project files from a source directory according to a manifest.
/// Reads each source file, computes its blob OID, maps it to the target path.
pub fn project(source_dir: &Path, manifest: &Manifest) -> Result<Projection, ProjectError> {
    let mut files = BTreeMap::new();

    for lens in &manifest.lenses {
        let source_path = source_dir.join(&lens.source);
        if !source_path.exists() {
            return Err(ProjectError::NotFound(lens.source.clone()));
        }
        let content = std::fs::read(&source_path)?;
        let oid = blob_oid_bytes(&content);
        files.insert(
            lens.target.clone(),
            ProjectedFile { content, oid },
        );
    }

    Ok(Projection { files })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn project_empty_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = Manifest::from_json(br#"{"lenses": []}"#).unwrap();
        let result = project(dir.path(), &manifest).unwrap();
        assert!(result.files.is_empty());
    }

    #[test]
    fn project_single_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("hello.txt"), b"hello world").unwrap();

        let manifest = Manifest::from_json(
            br#"{"lenses": [{"source": "hello.txt", "target": "out.txt"}]}"#,
        )
        .unwrap();

        let result = project(dir.path(), &manifest).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files["out.txt"].content, b"hello world");
        assert_eq!(
            result.files["out.txt"].oid,
            blob_oid_bytes(b"hello world")
        );
    }

    #[test]
    fn project_nested_source() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a/b")).unwrap();
        fs::write(dir.path().join("a/b/deep.md"), b"# Deep").unwrap();

        let manifest = Manifest::from_json(
            br#"{"lenses": [{"source": "a/b/deep.md", "target": "flat.md"}]}"#,
        )
        .unwrap();

        let result = project(dir.path(), &manifest).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files["flat.md"].content, b"# Deep");
    }

    #[test]
    fn project_missing_source_errors() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = Manifest::from_json(
            br#"{"lenses": [{"source": "nope.txt", "target": "out.txt"}]}"#,
        )
        .unwrap();

        let result = project(dir.path(), &manifest);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::NotFound(path) => assert_eq!(path, "nope.txt"),
            other => panic!("expected NotFound, got: {:?}", other),
        }
    }

    #[test]
    fn project_same_content_same_oid() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"same").unwrap();
        fs::write(dir.path().join("b.txt"), b"same").unwrap();

        let manifest = Manifest::from_json(
            br#"{"lenses": [
                {"source": "a.txt", "target": "x.txt"},
                {"source": "b.txt", "target": "y.txt"}
            ]}"#,
        )
        .unwrap();

        let result = project(dir.path(), &manifest).unwrap();
        assert_eq!(result.files["x.txt"].oid, result.files["y.txt"].oid);
    }
}
