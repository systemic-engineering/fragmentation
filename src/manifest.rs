use serde::Deserialize;

/// A single lens declaration: project a source path to an output path.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LensEntry {
    /// Path in the source repo (e.g., "visibility/public/README.md")
    pub source: String,
    /// Path in the output projection (e.g., "README.md")
    pub target: String,
}

/// A projection manifest: a list of lenses to apply.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub lenses: Vec<LensEntry>,
}

impl Manifest {
    /// Parse a manifest from JSON bytes.
    pub fn from_json(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_manifest() {
        let json = br#"{"lenses": []}"#;
        let manifest = Manifest::from_json(json).unwrap();
        assert_eq!(manifest.lenses.len(), 0);
    }

    #[test]
    fn parse_manifest_with_lenses() {
        let json = br#"{
            "lenses": [
                {"source": "visibility/public/README.md", "target": "README.md"},
                {"source": "visibility/public/CORPUS.md", "target": "CORPUS.md"}
            ]
        }"#;
        let manifest = Manifest::from_json(json).unwrap();
        assert_eq!(manifest.lenses.len(), 2);
        assert_eq!(manifest.lenses[0].source, "visibility/public/README.md");
        assert_eq!(manifest.lenses[0].target, "README.md");
    }

    #[test]
    fn parse_manifest_invalid_json() {
        let json = b"not json";
        assert!(Manifest::from_json(json).is_err());
    }
}
