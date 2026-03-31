//! Concurrent content-addressed store backed by DashMap.
//!
//! Lock-free reads, shard-locked writes. `&self` on all methods —
//! DashMap uses internal sharded RwLocks (default: CPU count * 4 shards).
//!
//! Content-addressing makes concurrent writes idempotent: same OID = same
//! bytes = no conflict. Two threads writing the same node produce the same
//! content OID and the same data. The second write is a no-op.
//!
//! The git2::Repository is NOT stored on the struct. Pass it to
//! `flush()` / `hydrate()` when persistence is needed. This keeps the
//! store `Send + Sync` unconditionally.

use dashmap::DashMap;

use crate::commit::Commit;
use crate::fragment::{content_oid, Fragmentable};
use crate::sha::HashAlg;

/// Concurrent content-addressed store.
///
/// `N`: node type (Fractal<String>, EigenSystem, etc.)
/// `H`: hash type (Sha, CoincidenceHash<N>, etc.)
///
/// All methods take `&self`. DashMap handles internal synchronization.
pub struct ConcurrentStore<N: Fragmentable + Clone, H: HashAlg = crate::sha::Sha> {
    objects: DashMap<String, N>,
    refs: DashMap<String, H>,
    commits: DashMap<String, Commit<N, H>>,
}

impl<N: Fragmentable + Clone, H: HashAlg> Default for ConcurrentStore<N, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Fragmentable + Clone, H: HashAlg> ConcurrentStore<N, H> {
    /// Create an empty concurrent store.
    pub fn new() -> Self {
        ConcurrentStore {
            objects: DashMap::new(),
            refs: DashMap::new(),
            commits: DashMap::new(),
        }
    }

    /// Number of stored objects.
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Write a node and all its children. Returns the root content OID.
    ///
    /// Content-addressing means concurrent writes are idempotent:
    /// same node produces the same OID and the same bytes. No conflict.
    pub fn write_tree(&self, node: &N) -> String {
        for child in node.children() {
            self.write_tree(child);
        }
        let oid = content_oid(node);
        self.objects
            .entry(oid.clone())
            .or_insert_with(|| node.clone());
        oid
    }

    /// Look up a node by its content OID.
    pub fn read_tree(&self, oid: &str) -> Option<N> {
        self.objects.get(oid).map(|r| r.value().clone())
    }

    /// Store a commit.
    pub fn write_commit(&self, commit: Commit<N, H>) {
        self.commits
            .insert(commit.sha().as_str().to_string(), commit);
    }

    /// Look up a commit by its hash.
    pub fn read_commit(&self, sha: &H) -> Option<Commit<N, H>> {
        self.commits.get(sha.as_str()).map(|r| r.value().clone())
    }

    /// Point a ref at a hash.
    pub fn update_ref(&self, key: &str, sha: H) {
        self.refs.insert(key.to_string(), sha);
    }

    /// Resolve a ref to a hash.
    pub fn resolve_ref(&self, key: &str) -> Option<H> {
        self.refs.get(key).map(|r| r.value().clone())
    }

    /// List all ref names.
    pub fn ref_names(&self) -> Vec<String> {
        self.refs.iter().map(|r| r.key().clone()).collect()
    }

    /// List all object OIDs.
    pub fn keys(&self) -> Vec<String> {
        self.objects.iter().map(|r| r.key().clone()).collect()
    }
}

// ---------------------------------------------------------------------------
// Git integration: flush / hydrate
// ---------------------------------------------------------------------------

#[cfg(feature = "git")]
impl<N: Fragmentable + Clone, H: HashAlg> ConcurrentStore<N, H>
where
    N: crate::fragment::Reconstructable,
    N::Data: crate::encoding::Decode,
{
    /// Write all in-memory objects to the git ODB and update refs/store/index.
    /// Returns the number of objects written.
    ///
    /// The git2::Repository is passed in — not stored on the struct.
    /// This keeps ConcurrentStore unconditionally Send + Sync.
    pub fn flush(&self, repo: &git2::Repository) -> usize {
        let mut count = 0;

        // Write all objects to git ODB.
        for entry in self.objects.iter() {
            if crate::git::write_node(repo, entry.value()).is_ok() {
                count += 1;
            }
        }

        // Collapse all refs into a single git tree at refs/store/index.
        match self.collapse_index(repo) {
            Ok(tree_oid) => {
                if let Err(e) = repo.reference("refs/store/index", tree_oid, true, "collapse") {
                    eprintln!("warning: failed to update refs/store/index: {e}");
                }
            }
            Err(e) => {
                eprintln!("warning: collapse_index failed: {e}");
            }
        }

        count
    }

    /// Load refs from git (refs/store/index) into memory.
    ///
    /// Populates the ref map from the git index tree. Objects are loaded
    /// lazily on read_tree miss (if a git read-through is wired up by
    /// the consumer).
    pub fn hydrate(&self, repo: &git2::Repository) {
        let oid = match repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.target())
        {
            Some(oid) => oid,
            None => return,
        };

        if let Ok(tree) = repo.find_tree(oid) {
            for entry in tree.iter() {
                if let Some(name) = entry.name() {
                    // The blob contains the ref value (content OID) as text.
                    if let Ok(blob) = repo.find_blob(entry.id()) {
                        if let Ok(content) = std::str::from_utf8(blob.content()) {
                            self.refs.insert(name.to_string(), H::from_hex(content));
                        }
                    }
                }
            }
        }
    }

    /// Collapse: serialize all in-memory refs as entries in one git tree.
    /// Returns the tree OID. Merges with any existing index on disk.
    fn collapse_index(&self, repo: &git2::Repository) -> Result<git2::Oid, git2::Error> {
        let existing_tree = repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.peel_to_tree().ok());
        let mut builder = repo.treebuilder(existing_tree.as_ref())?;

        for entry in self.refs.iter() {
            // Write the ref value as a blob so the OID exists in the ODB.
            let blob_oid = repo.blob(entry.value().as_str().as_bytes())?;
            builder.insert(entry.key().as_str(), blob_oid, 0o100644)?;
        }

        builder.write()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;
    use crate::fragment::Fractal;
    use crate::sha::Sha;

    fn test_fractal() -> Fractal<String> {
        encoding::encode("hello world")
    }

    // -- Construction --

    #[test]
    fn new_store_is_empty() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    // -- write_tree / read_tree --

    #[test]
    fn write_and_read_tree() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store.write_tree(&fractal);
        let read_back = store.read_tree(&oid).expect("should find tree");
        assert_eq!(read_back, fractal);
    }

    #[test]
    fn read_tree_miss_returns_none() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        assert!(store.read_tree("nonexistent").is_none());
    }

    #[test]
    fn write_tree_is_idempotent() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid1 = store.write_tree(&fractal);
        let oid2 = store.write_tree(&fractal);
        assert_eq!(oid1, oid2);
    }

    #[test]
    fn write_tree_stores_children() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let _oid = store.write_tree(&fractal);
        // The encoding tree has children (paragraphs, sentences, words, chars).
        // All should be stored.
        assert!(store.len() > 1);
    }

    #[test]
    fn len_counts_all_objects() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        assert_eq!(store.len(), 0);
        let fractal = test_fractal();
        store.write_tree(&fractal);
        let count = store.len();
        assert!(count > 0);
        // Writing the same tree again should not increase count.
        store.write_tree(&fractal);
        assert_eq!(store.len(), count);
    }

    // -- refs --

    #[test]
    fn update_and_resolve_ref() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let sha = Sha("abc123".into());
        store.update_ref("refs/heads/main", sha.clone());
        assert_eq!(store.resolve_ref("refs/heads/main"), Some(sha));
    }

    #[test]
    fn resolve_ref_miss_returns_none() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        assert!(store.resolve_ref("refs/heads/main").is_none());
    }

    #[test]
    fn ref_names_lists_all() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        store.update_ref("refs/heads/main", Sha("abc".into()));
        store.update_ref("grammar/test", Sha("def".into()));
        let mut names = store.ref_names();
        names.sort();
        assert_eq!(names, vec!["grammar/test", "refs/heads/main"]);
    }

    // -- commits --

    #[test]
    fn write_and_read_commit() {
        use crate::commit::Draft;
        use crate::witnessed::Committer;

        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        // We need a mutable store for Draft::commit, but we can test
        // the concurrent store's commit read/write directly.
        let mut mem_store = crate::store::Store::<Fractal<String>>::new();
        let committer = Committer::new("Test", "test@test.com");
        let draft = Draft::root("test", fractal);
        let commit = draft.commit(&mut mem_store, committer, "1234567890 +0000");
        let sha = commit.sha().clone();
        store.write_commit(commit.clone());
        assert_eq!(store.read_commit(&sha), Some(commit));
    }

    #[test]
    fn read_commit_miss_returns_none() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        assert!(store.read_commit(&Sha("anything".into())).is_none());
    }

    // -- keys --

    #[test]
    fn keys_lists_all_oids() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        assert!(store.keys().is_empty());
        let fractal = test_fractal();
        store.write_tree(&fractal);
        let keys = store.keys();
        assert!(!keys.is_empty());
    }

    // -- &self methods (compile-time proof of shared-reference writes) --

    #[test]
    fn all_methods_take_shared_ref() {
        // This test proves at compile time that write methods take &self.
        // If any method required &mut self, this would fail to compile.
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();

        let _oid = store.write_tree(&fractal);
        let _node = store.read_tree("anything");
        store.update_ref("test", Sha("abc".into()));
        let _r = store.resolve_ref("test");
        let _len = store.len();
        let _empty = store.is_empty();
        let _names = store.ref_names();
        let _keys = store.keys();
    }

    // -- git integration --

    #[cfg(feature = "git")]
    #[test]
    fn flush_writes_objects_to_git() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        store.write_tree(&fractal);

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let count = store.flush(&repo);
        assert!(count > 0);
    }

    #[cfg(feature = "git")]
    #[test]
    fn flush_updates_index_ref() {
        let store = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store.write_tree(&fractal);
        store.update_ref("test_ref", Sha(oid));

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        store.flush(&repo);

        // The index ref should exist in git.
        let reference = repo.find_reference("refs/store/index");
        assert!(reference.is_ok());
    }

    #[cfg(feature = "git")]
    #[test]
    fn hydrate_loads_refs_from_git() {
        // First store: write and flush.
        let store1 = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store1.write_tree(&fractal);
        store1.update_ref("eigen_test", Sha(oid.clone()));

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        store1.flush(&repo);

        // Second store: hydrate from git.
        let store2 = ConcurrentStore::<Fractal<String>>::new();
        store2.hydrate(&repo);
        let resolved = store2.resolve_ref("eigen_test");
        assert_eq!(resolved, Some(Sha(oid)));
    }

    #[cfg(feature = "git")]
    #[test]
    fn flush_then_hydrate_roundtrip() {
        let store1 = ConcurrentStore::<Fractal<String>>::new();
        let fractal = test_fractal();
        let oid = store1.write_tree(&fractal);
        store1.update_ref("ref_a", Sha(oid.clone()));
        store1.update_ref("ref_b", Sha("deadbeef".into()));

        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        store1.flush(&repo);

        let store2 = ConcurrentStore::<Fractal<String>>::new();
        store2.hydrate(&repo);

        assert_eq!(store2.resolve_ref("ref_a"), Some(Sha(oid)));
        assert_eq!(store2.resolve_ref("ref_b"), Some(Sha("deadbeef".into())));
    }
}
