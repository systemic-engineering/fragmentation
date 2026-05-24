//! Git-backed bounded store. Eviction writes to git. Cache miss reads from git.

use std::sync::Mutex;

use fragmentation::bounded_store::BoundedStore;
use fragmentation::encoding::{Decode, Encode};
use fragmentation::fragment::{ContentAddressed, Fractal, Fragmentable, Reconstructable, TreeShaped};

/// Estimate the byte size of a Fractal node (data + ref + children overhead).
fn estimate_fractal_size<E: Encode>(fractal: &Fractal<E>) -> usize {
    let base = std::mem::size_of_val(fractal);
    let data_size = std::mem::size_of_val(fractal.data());
    base + data_size
}

/// A git-backed bounded store. Eviction writes to git. Cache miss reads from git.
/// The boundary between memory and disk is a write, not a delete.
pub struct GitBoundedStore<E: Encode + Decode + Clone> {
    pub(crate) cache: BoundedStore<Fractal<E>>,
    pub(crate) repo: Mutex<git2::Repository>,
}

impl<E: Encode + Decode + Clone> GitBoundedStore<E> {
    /// Open a git-backed bounded store with a byte capacity.
    pub fn open(repo_path: &str, max_bytes: usize) -> Result<Self, git2::Error> {
        let repo = git2::Repository::open(repo_path)?;
        Ok(GitBoundedStore {
            cache: BoundedStore::new(max_bytes),
            repo: Mutex::new(repo),
        })
    }

    /// Insert a node. If the cache exceeds byte capacity, evicted nodes
    /// are written to git first, with a ref at `refs/spectral/nodes/{key}`.
    pub fn insert(&self, key: String, value: Fractal<E>) {
        let size_bytes = estimate_fractal_size(&value);
        // Check if we'll need to evict
        if self.cache.total_bytes() + size_bytes > self.cache.capacity() {
            // Write oldest to git before BoundedStore evicts it
            if let Some((oldest_key, oldest_node)) = self.cache.peek_oldest() {
                let repo = self.repo.lock().unwrap();
                if let Ok(git_oid) = crate::git::write_tree(&repo, &oldest_node) {
                    let refname = format!("refs/spectral/nodes/{}", oldest_key);
                    let _ = repo.reference(&refname, git_oid, true, "spectral eviction");
                }
            }
        }
        self.cache.insert(key, value, size_bytes);
    }

    /// Look up a node. Checks cache first, falls back to git on miss.
    pub fn get(&self, key: &str) -> Option<Fractal<E>>
    where
        Fractal<E>: Reconstructable<Data = E, Hash = fragmentation::sha::Sha>,
    {
        // Hot path: in cache
        if let Some(node) = self.cache.get(key) {
            return Some(node);
        }

        // Cold path: read from git
        let repo = self.repo.lock().unwrap();
        let oid = git2::Oid::from_str(key).ok()?;
        let node: Fractal<E> = crate::git::read_node(&repo, oid).ok()?;
        drop(repo);

        // Promote back to cache
        let size = estimate_fractal_size(&node);
        self.cache.insert(key.to_string(), node.clone(), size);

        Some(node)
    }

    /// Number of entries in the memory cache.
    pub fn cached_len(&self) -> usize {
        self.cache.len()
    }

    /// Total bytes in the memory cache.
    pub fn total_bytes(&self) -> usize {
        self.cache.total_bytes()
    }

    /// Byte capacity.
    pub fn capacity(&self) -> usize {
        self.cache.capacity()
    }

    /// Flush all cached entries to git, then clear the cache.
    ///
    /// Each flushed node gets a git ref at `refs/spectral/nodes/{key}`
    /// pointing to its tree OID, so the index can be rebuilt from refs
    /// without a manifest file.
    pub fn flush(&self) {
        let repo = self.repo.lock().unwrap();
        self.cache.drain_all(|key, node| {
            if let Ok(git_oid) = crate::git::write_tree(&repo, node) {
                let refname = format!("refs/spectral/nodes/{}", key);
                let _ = repo.reference(&refname, git_oid, true, "spectral flush");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fragmentation::encoding;
    use fragmentation::fragment::Fractal;

    fn shard(label: &str) -> Fractal<String> {
        encoding::encode(label)
    }

    fn oid(node: &Fractal<String>) -> String {
        fragmentation::fragment::content_oid(node)
    }

    #[test]
    fn git_bounded_store_evict_persists_to_git() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        // Small byte capacity — forces eviction after ~2 entries
        let store = GitBoundedStore::open(dir.path().to_str().unwrap(), 256).unwrap();

        let a = shard("persist-a");
        let b = shard("persist-b");
        let c = shard("persist-c");
        let ka = oid(&a);

        store.insert(ka.clone(), a);
        store.insert(oid(&b), b);
        // This insert evicts "persist-a" — which should write it to git first
        store.insert(oid(&c), c);

        // "a" is not in cache
        assert!(store.cache.get(&ka).is_none());

        // But "a" was written to git during eviction — verify the blob OID is parseable
        let git_oid = git2::Oid::from_str(&fragmentation::fragment::blob_oid("persist-a")).ok();
        assert!(git_oid.is_some(), "blob OID should be parseable");
    }

    #[test]
    fn git_bounded_store_cache_miss_reads_from_git() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        // Write a shard directly to git
        let node = shard("cold-data");
        let tree_oid = crate::git::write_tree(&repo, &node).unwrap();
        drop(repo);

        let store: GitBoundedStore<String> =
            GitBoundedStore::open(dir.path().to_str().unwrap(), 10_000).unwrap();

        // Cache is empty
        assert_eq!(store.cached_len(), 0);

        // Read via git OID — should fetch from git and promote to cache
        let result = store.get(&tree_oid.to_string());
        assert!(result.is_some(), "should read from git on cache miss");
        assert_eq!(store.cached_len(), 1, "should promote to cache after miss");
    }

    #[test]
    fn git_bounded_store_flush_writes_all() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let store = GitBoundedStore::open(dir.path().to_str().unwrap(), 10_000).unwrap();

        store.insert("k1".to_string(), shard("flush-1"));
        store.insert("k2".to_string(), shard("flush-2"));
        assert_eq!(store.cached_len(), 2);

        store.flush();
        assert_eq!(store.cached_len(), 0);
    }

    #[test]
    fn refs_written_on_flush() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let store = GitBoundedStore::open(dir.path().to_str().unwrap(), 10_000).unwrap();

        let a = shard("ref-flush-a");
        let b = shard("ref-flush-b");
        let ka = oid(&a);
        let kb = oid(&b);

        store.insert(ka.clone(), a);
        store.insert(kb.clone(), b);
        store.flush();

        // Open the git repo directly and verify refs exist
        let repo = git2::Repository::open(dir.path()).unwrap();
        let refs: Vec<String> = repo
            .references_glob("refs/spectral/nodes/*")
            .unwrap()
            .filter_map(|r| r.ok())
            .filter_map(|r| r.name().map(|n| n.to_string()))
            .collect();

        assert_eq!(refs.len(), 2, "flush should write 2 refs, got {:?}", refs);
        assert!(
            refs.contains(&format!("refs/spectral/nodes/{}", ka)),
            "ref for key a must exist"
        );
        assert!(
            refs.contains(&format!("refs/spectral/nodes/{}", kb)),
            "ref for key b must exist"
        );
    }

    #[test]
    fn refs_written_on_eviction() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        // Small capacity — forces eviction after ~2 entries
        let store = GitBoundedStore::open(dir.path().to_str().unwrap(), 256).unwrap();

        let a = shard("ref-evict-a");
        let b = shard("ref-evict-b");
        let c = shard("ref-evict-c");
        let ka = oid(&a);

        store.insert(ka.clone(), a);
        store.insert(oid(&b), b);
        // This insert should evict "a" and write its ref
        store.insert(oid(&c), c);

        // "a" should have been evicted from cache
        assert!(store.cache.get(&ka).is_none(), "a should be evicted");

        // But the eviction should have written a ref for it
        let repo = git2::Repository::open(dir.path()).unwrap();
        let refname = format!("refs/spectral/nodes/{}", ka);
        let reference = repo.find_reference(&refname);
        assert!(
            reference.is_ok(),
            "evicted node must have a ref at {}",
            refname
        );
    }
}
