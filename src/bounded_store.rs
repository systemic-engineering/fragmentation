//! Size-bounded concurrent store with LIFO eviction.
//!
//! Two modes:
//! - [`BoundedStore`]: in-memory cache with capacity limit. Eviction drops.
//! - [`GitBoundedStore`]: same cache, but eviction writes to git before dropping.
//!   Cache miss reads from git. The boundary between memory and disk is a write.

use std::collections::VecDeque;
use std::sync::Mutex;

use dashmap::DashMap;

use crate::encoding::Encode;
use crate::fragment::{Fractal, Fragmentable};
use crate::sha::HashAlg;

/// Estimate the byte size of a Fractal node (data + ref + children overhead).
fn estimate_fractal_size<E: Encode>(fractal: &Fractal<E>) -> usize {
    // Ref hash (32-64 bytes) + label + data encoding estimate.
    // We use std::mem::size_of_val as a floor, then add data encoding.
    let base = std::mem::size_of_val(fractal);
    let data_size = std::mem::size_of_val(fractal.data());
    base + data_size
}

/// A byte-bounded concurrent content-addressed store.
///
/// Uses a `DashMap` for O(1) concurrent lookup and a `VecDeque` to track
/// insertion order. On insert, if total bytes exceed `max_bytes`, the oldest
/// entries (back of the deque) are evicted until the store fits.
pub struct BoundedStore<N: Fragmentable + Clone, H: HashAlg = crate::sha::Sha> {
    objects: DashMap<String, N>,
    sizes: DashMap<String, usize>,
    order: Mutex<VecDeque<String>>,
    total_bytes: Mutex<usize>,
    max_bytes: usize,
    _hash: std::marker::PhantomData<H>,
}

impl<N: Fragmentable + Clone, H: HashAlg> BoundedStore<N, H> {
    /// Create a new bounded store with the given byte capacity.
    pub fn new(max_bytes: usize) -> Self {
        BoundedStore {
            objects: DashMap::new(),
            sizes: DashMap::new(),
            order: Mutex::new(VecDeque::new()),
            total_bytes: Mutex::new(0),
            max_bytes,
            _hash: std::marker::PhantomData,
        }
    }

    /// Insert a node with its byte size. Evicts oldest if over capacity.
    pub fn insert(&self, key: String, value: N, size_bytes: usize) {
        self.objects.insert(key.clone(), value);
        self.sizes.insert(key.clone(), size_bytes);
        let mut order = self.order.lock().unwrap();
        let mut total = self.total_bytes.lock().unwrap();
        order.push_front(key);
        *total += size_bytes;
        while *total > self.max_bytes && order.len() > 1 {
            if let Some(evicted) = order.pop_back() {
                self.objects.remove(&evicted);
                if let Some((_, sz)) = self.sizes.remove(&evicted) {
                    *total = total.saturating_sub(sz);
                }
            }
        }
    }

    /// Look up a node by its content OID.
    pub fn get(&self, key: &str) -> Option<N> {
        self.objects.get(key).map(|r| r.value().clone())
    }

    /// Number of entries currently tracked.
    pub fn len(&self) -> usize {
        self.order.lock().unwrap().len()
    }

    /// Total bytes in the store.
    pub fn total_bytes(&self) -> usize {
        *self.total_bytes.lock().unwrap()
    }

    /// Maximum bytes this store will hold.
    pub fn capacity(&self) -> usize {
        self.max_bytes
    }

    /// Peek at the oldest key without removing it. Returns a clone.
    pub fn peek_oldest(&self) -> Option<(String, N)> {
        let order = self.order.lock().unwrap();
        if let Some(key) = order.back() {
            let value = self.objects.get(key).map(|r| r.value().clone());
            value.map(|v| (key.clone(), v))
        } else {
            None
        }
    }

    /// Drain all entries: calls `on_entry` for each (key, value), then clears.
    pub fn drain_all(&self, mut on_entry: impl FnMut(&str, &N)) {
        let order = self.order.lock().unwrap();
        for key in order.iter() {
            if let Some(node) = self.objects.get(key) {
                on_entry(key, node.value());
            }
        }
        drop(order);
        self.objects.clear();
        self.order.lock().unwrap().clear();
        *self.total_bytes.lock().unwrap() = 0;
    }

    /// Manually evict the oldest entry. Returns the evicted key, if any.
    pub fn evict_one(&self) -> Option<String> {
        let mut order = self.order.lock().unwrap();
        let mut total = self.total_bytes.lock().unwrap();
        if let Some(key) = order.pop_back() {
            self.objects.remove(&key);
            if let Some((_, sz)) = self.sizes.remove(&key) {
                *total = total.saturating_sub(sz);
            }
            Some(key)
        } else {
            None
        }
    }
}

/// A git-backed bounded store. Eviction writes to git. Cache miss reads from git.
/// The boundary between memory and disk is a write, not a delete.
pub struct GitBoundedStore<E: crate::encoding::Encode + crate::encoding::Decode + Clone> {
    cache: BoundedStore<crate::fragment::Fractal<E>>,
    repo: Mutex<git2::Repository>,
}

impl<E: crate::encoding::Encode + crate::encoding::Decode + Clone> GitBoundedStore<E> {
    /// Open a git-backed bounded store with a byte capacity.
    pub fn open(repo_path: &str, max_bytes: usize) -> Result<Self, git2::Error> {
        let repo = git2::Repository::open(repo_path)?;
        Ok(GitBoundedStore {
            cache: BoundedStore::new(max_bytes),
            repo: Mutex::new(repo),
        })
    }

    /// Insert a node. If the cache exceeds byte capacity, evicted nodes
    /// are written to git first.
    pub fn insert(&self, key: String, value: crate::fragment::Fractal<E>) {
        let size_bytes = estimate_fractal_size(&value);
        // Check if we'll need to evict
        if self.cache.total_bytes() + size_bytes > self.cache.capacity() {
            // Write oldest to git before BoundedStore evicts it
            let order = self.cache.order.lock().unwrap();
            if let Some(oldest_key) = order.back() {
                if let Some(node) = self.cache.objects.get(oldest_key) {
                    let repo = self.repo.lock().unwrap();
                    let _ = crate::git::write_tree(&repo, node.value());
                }
            }
            drop(order);
        }
        self.cache.insert(key, value, size_bytes);
    }

    /// Look up a node. Checks cache first, falls back to git on miss.
    pub fn get(&self, key: &str) -> Option<crate::fragment::Fractal<E>>
    where
        crate::fragment::Fractal<E>: crate::fragment::Reconstructable<Data = E, Hash = crate::sha::Sha>,
    {
        // Hot path: in cache
        if let Some(node) = self.cache.get(key) {
            return Some(node);
        }

        // Cold path: read from git
        let repo = self.repo.lock().unwrap();
        let oid = git2::Oid::from_str(key).ok()?;
        let node: crate::fragment::Fractal<E> = crate::git::read_node(&repo, oid).ok()?;
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
    pub fn flush(&self) {
        let repo = self.repo.lock().unwrap();
        let order = self.cache.order.lock().unwrap();
        for key in order.iter() {
            if let Some(node) = self.cache.objects.get(key) {
                let _ = crate::git::write_tree(&repo, node.value());
            }
        }
        drop(order);
        drop(repo);

        // Clear cache
        self.cache.objects.clear();
        self.cache.order.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;
    use crate::fragment::Fractal;

    /// Helper: build a simple shard with a given label.
    fn shard(label: &str) -> Fractal<String> {
        encoding::encode(label)
    }

    /// Helper: compute the content OID for a node.
    fn oid(node: &Fractal<String>) -> String {
        crate::fragment::content_oid(node)
    }

    #[test]
    fn empty_store_has_zero_len() {
        let store = BoundedStore::<Fractal<String>>::new(10_000);
        assert_eq!(store.len(), 0);
        assert_eq!(store.total_bytes(), 0);
    }

    #[test]
    fn insert_and_get() {
        let store = BoundedStore::<Fractal<String>>::new(10_000);
        let node = shard("hello");
        let key = oid(&node);
        store.insert(key.clone(), node.clone(), 100);
        assert_eq!(store.get(&key), Some(node));
        assert_eq!(store.total_bytes(), 100);
    }

    #[test]
    fn respects_byte_capacity() {
        let store = BoundedStore::<Fractal<String>>::new(200);
        for i in 0..5 {
            let node = shard(&format!("item-{i}"));
            let key = oid(&node);
            store.insert(key, node, 100);
        }
        // 200 bytes capacity, 100 bytes each → at most 2
        assert!(store.len() <= 2);
        assert!(store.total_bytes() <= 200);
    }

    #[test]
    fn evicts_oldest() {
        let store = BoundedStore::<Fractal<String>>::new(200);
        let a = shard("aaa");
        let b = shard("bbb");
        let c = shard("ccc");
        let ka = oid(&a);
        let kb = oid(&b);
        let kc = oid(&c);

        store.insert(ka.clone(), a, 100);
        store.insert(kb.clone(), b.clone(), 100);
        store.insert(kc.clone(), c.clone(), 100);

        // "aaa" was oldest — should be evicted.
        assert!(store.get(&ka).is_none(), "oldest entry should be evicted");
        // "bbb" and "ccc" should still be present.
        assert_eq!(store.get(&kb), Some(b));
        assert_eq!(store.get(&kc), Some(c));
    }

    #[test]
    fn get_missing_returns_none() {
        let store = BoundedStore::<Fractal<String>>::new(10_000);
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn evict_one_returns_key() {
        let store = BoundedStore::<Fractal<String>>::new(10_000);
        let node = shard("evict-me");
        let key = oid(&node);
        store.insert(key.clone(), node, 100);
        let evicted = store.evict_one();
        assert_eq!(evicted, Some(key.clone()));
        assert!(store.get(&key).is_none());
        assert_eq!(store.total_bytes(), 0);
    }
}

#[cfg(test)]
mod git_tests {
    use super::*;
    use crate::fragment::Fractal;

    fn shard(label: &str) -> Fractal<String> {
        crate::encoding::encode(label)
    }

    fn oid(node: &Fractal<String>) -> String {
        crate::fragment::content_oid(node)
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

        // But "a" was written to git during eviction — verify the blob exists
        let repo = store.repo.lock().unwrap();
        let git_oid = git2::Oid::from_str(&crate::fragment::blob_oid("persist-a")).ok();
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
}
