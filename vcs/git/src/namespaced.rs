//! NamespacedGitStore — a FrgmntStore living inside `.git/<namespace>/`.
//!
//! This bridges mirror's content-addressed world and git's working tree.
//! The store lives inside `.git/` so it doesn't pollute the working tree,
//! but uses the `.frgmnt` file format (fan-out objects + refs) rather than
//! git's own object database.
//!
//! ```text
//! .git/mirror/          ← NamespacedGitStore::open(repo, "mirror")
//!   objects/
//!     ab/cdef1234...
//!   refs/
//!     boot
//! ```

use std::path::{Path, PathBuf};

use fragmentation::fragment::Fractal;
use fragmentation::frgmnt_store::FrgmntStore;

/// Default cache size: 16 MiB.
const DEFAULT_CACHE_BYTES: usize = 16 * 1024 * 1024;

/// A FrgmntStore that lives inside a git repository's `.git/` directory.
///
/// Namespace determines the subdirectory:
/// - `"mirror"` → `.git/mirror/`
/// - `"spectral"` → `.git/spectral/`
/// - `"notes/mirror"` → `.git/notes/mirror/`
///
/// The store uses the `.frgmnt` file format (content-addressed objects
/// with fan-out, named refs) but lives inside `.git/` so it's invisible
/// to the working tree and travels with clones (if refs are pushed).
pub struct NamespacedGitStore {
    store: FrgmntStore<Fractal<String>>,
    git_dir: PathBuf,
    namespace: String,
}

/// Error type for NamespacedGitStore operations.
#[derive(Debug)]
pub enum NamespacedStoreError {
    /// Not inside a git repository.
    NotAGitRepo(PathBuf),
    /// Store initialization failed.
    StoreInit(fragmentation::frgmnt_store::Error),
    /// I/O error.
    Io(std::io::Error),
}

impl std::fmt::Display for NamespacedStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NamespacedStoreError::NotAGitRepo(p) => {
                write!(f, "not a git repository: {}", p.display())
            }
            NamespacedStoreError::StoreInit(e) => write!(f, "store init: {}", e),
            NamespacedStoreError::Io(e) => write!(f, "io: {}", e),
        }
    }
}

impl std::error::Error for NamespacedStoreError {}

impl From<std::io::Error> for NamespacedStoreError {
    fn from(e: std::io::Error) -> Self {
        NamespacedStoreError::Io(e)
    }
}

impl From<fragmentation::frgmnt_store::Error> for NamespacedStoreError {
    fn from(e: fragmentation::frgmnt_store::Error) -> Self {
        NamespacedStoreError::StoreInit(e)
    }
}

impl NamespacedGitStore {
    /// Open or create a namespaced store inside a git repository.
    ///
    /// Discovers the `.git/` directory from `repo_path` (walking up),
    /// then creates `.git/<namespace>/` with the frgmnt store structure.
    pub fn open(repo_path: &Path, namespace: &str) -> Result<Self, NamespacedStoreError> {
        Self::open_with_cache(repo_path, namespace, DEFAULT_CACHE_BYTES)
    }

    /// Open with a custom cache size.
    pub fn open_with_cache(
        repo_path: &Path,
        namespace: &str,
        cache_bytes: usize,
    ) -> Result<Self, NamespacedStoreError> {
        let git_dir = find_git_dir(repo_path)?;
        let ns_dir = git_dir.join(namespace);
        let store = FrgmntStore::open(ns_dir.to_str().unwrap_or(""), cache_bytes)?;
        Ok(NamespacedGitStore {
            store,
            git_dir: ns_dir,
            namespace: namespace.to_string(),
        })
    }

    /// The store directory path (e.g., `.git/mirror/`).
    pub fn path(&self) -> &Path {
        &self.git_dir
    }

    /// The namespace name.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Access the underlying FrgmntStore.
    pub fn store(&self) -> &FrgmntStore<Fractal<String>> {
        &self.store
    }

    /// Insert a fragment into the cache.
    pub fn insert(&self, key: String, value: Fractal<String>, size_bytes: usize) {
        self.store.insert(key, value, size_bytes);
    }

    /// Get a fragment from cache only.
    pub fn get(&self, key: &str) -> Option<Fractal<String>> {
        self.store.get(key)
    }

    /// Insert with disk persistence (eviction writes to disk).
    pub fn insert_persistent(&self, key: String, value: Fractal<String>, size_bytes: usize) {
        self.store.insert_persistent(key, value, size_bytes);
    }

    /// Get with disk fallback.
    pub fn get_persistent(&self, key: &str) -> Option<Fractal<String>> {
        self.store.get_persistent(key)
    }

    /// Write a named ref.
    pub fn set_ref(&self, name: &str, oid: &str) -> Result<(), fragmentation::frgmnt_store::Error> {
        self.store.set_ref(name, oid)
    }

    /// Read a named ref.
    pub fn get_ref(&self, name: &str) -> Option<String> {
        self.store.get_ref(name)
    }

    /// Flush all cached entries to disk.
    pub fn flush(&self) {
        self.store.flush();
    }

    /// Number of cached entries.
    pub fn cached_len(&self) -> usize {
        self.store.cached_len()
    }
}

/// Find the `.git/` directory from a path, walking up.
fn find_git_dir(path: &Path) -> Result<PathBuf, NamespacedStoreError> {
    let mut current = path.to_path_buf();
    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return Ok(git_dir);
        }
        // Handle git worktrees: .git is a file containing "gitdir: <path>"
        if git_dir.is_file() {
            if let Ok(content) = std::fs::read_to_string(&git_dir) {
                if let Some(gitdir) = content.strip_prefix("gitdir: ") {
                    let gitdir = gitdir.trim();
                    let resolved = if Path::new(gitdir).is_absolute() {
                        PathBuf::from(gitdir)
                    } else {
                        current.join(gitdir)
                    };
                    if resolved.is_dir() {
                        return Ok(resolved);
                    }
                }
            }
        }
        if !current.pop() {
            return Err(NamespacedStoreError::NotAGitRepo(path.to_path_buf()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fragmentation::encoding;
    use fragmentation::fragment::{self, ContentAddressed, Fractal, Fragmentable, TreeShaped};

    fn shard(label: &str) -> Fractal<String> {
        encoding::encode(label)
    }

    fn oid(node: &Fractal<String>) -> String {
        fragment::content_oid(node)
    }

    #[test]
    fn open_creates_namespace_dir() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let store = NamespacedGitStore::open(dir.path(), "mirror").unwrap();
        assert!(store.path().exists());
        assert!(store.path().join("objects").exists());
        assert!(store.path().join("refs").exists());
        assert_eq!(store.namespace(), "mirror");
    }

    #[test]
    fn open_nested_namespace() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let store = NamespacedGitStore::open(dir.path(), "notes/mirror").unwrap();
        assert!(store.path().exists());
        assert_eq!(store.namespace(), "notes/mirror");
    }

    #[test]
    fn not_a_git_repo_fails() {
        let dir = tempfile::tempdir().unwrap();
        let result = NamespacedGitStore::open(dir.path(), "mirror");
        assert!(result.is_err());
    }

    #[test]
    fn insert_and_get() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let store = NamespacedGitStore::open(dir.path(), "mirror").unwrap();

        let node = shard("hello");
        let key = oid(&node);
        store.insert(key.clone(), node.clone(), 100);
        let got = store.get(&key);
        assert!(got.is_some());
        assert_eq!(got.unwrap().data(), node.data());
    }

    #[test]
    fn persistent_insert_and_get() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let store = NamespacedGitStore::open_with_cache(dir.path(), "mirror", 100).unwrap();

        let a = shard("persist-a");
        let b = shard("persist-b");
        let c = shard("persist-c");
        let ka = oid(&a);

        store.insert_persistent(ka.clone(), a, 50);
        store.insert_persistent(oid(&b), b, 50);
        store.insert_persistent(oid(&c), c, 50);

        let got = store.get_persistent(&ka);
        assert!(got.is_some(), "evicted entry should be readable from disk");
    }

    #[test]
    fn refs_work() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let store = NamespacedGitStore::open(dir.path(), "mirror").unwrap();

        store.set_ref("boot", "abc123").unwrap();
        assert_eq!(store.get_ref("boot").as_deref(), Some("abc123"));
    }

    #[test]
    fn flush_writes_all() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let store = NamespacedGitStore::open(dir.path(), "spectral").unwrap();

        let a = shard("flush-a");
        let b = shard("flush-b");
        store.insert(oid(&a), a, 50);
        store.insert(oid(&b), b, 50);
        assert_eq!(store.cached_len(), 2);

        store.flush();
        assert_eq!(store.cached_len(), 0);
    }

    #[test]
    fn discovers_git_dir_from_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let sub = dir.path().join("deep").join("nested");
        std::fs::create_dir_all(&sub).unwrap();

        let store = NamespacedGitStore::open(&sub, "mirror").unwrap();
        assert!(store.path().exists());
    }

    #[test]
    fn multiple_namespaces_coexist() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let mirror = NamespacedGitStore::open(dir.path(), "mirror").unwrap();
        let spectral = NamespacedGitStore::open(dir.path(), "spectral").unwrap();

        mirror.set_ref("head", "mirror-oid").unwrap();
        spectral.set_ref("head", "spectral-oid").unwrap();

        assert_eq!(mirror.get_ref("head").as_deref(), Some("mirror-oid"));
        assert_eq!(spectral.get_ref("head").as_deref(), Some("spectral-oid"));
    }
}
