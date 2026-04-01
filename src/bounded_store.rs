//! Size-bounded concurrent store with LIFO eviction.
//!
//! Wraps [`ConcurrentStore`] with a capacity limit. When the store exceeds
//! `max_entries`, the oldest entry (back of the insertion-order deque) is
//! evicted. Evicted data is not lost if backed by git — it can be re-fetched
//! on cache miss.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::concurrent_store::ConcurrentStore;
use crate::fragment::Fragmentable;
use crate::sha::HashAlg;

/// A size-bounded concurrent content-addressed store.
///
/// Uses a `VecDeque` to track insertion order. On insert, if the number of
/// tracked entries exceeds `max_entries`, the oldest entry (back of the deque)
/// is evicted from both the deque and the underlying `ConcurrentStore`.
pub struct BoundedStore<N: Fragmentable + Clone, H: HashAlg = crate::sha::Sha> {
    inner: ConcurrentStore<N, H>,
    order: Mutex<VecDeque<String>>,
    max_entries: usize,
}

impl<N: Fragmentable + Clone, H: HashAlg> BoundedStore<N, H> {
    /// Create a new bounded store with the given capacity.
    pub fn new(max_entries: usize) -> Self {
        BoundedStore {
            inner: ConcurrentStore::new(),
            order: Mutex::new(VecDeque::with_capacity(max_entries)),
            max_entries,
        }
    }

    /// Insert a node, tracking it by key. Evicts oldest if over capacity.
    pub fn insert(&self, key: String, value: N) {
        let _ = (key, value);
        todo!()
    }

    /// Look up a node by its content OID.
    pub fn get(&self, key: &str) -> Option<N> {
        let _ = key;
        todo!()
    }

    /// Number of entries currently tracked.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// Maximum number of entries this store will hold.
    pub fn capacity(&self) -> usize {
        todo!()
    }

    /// Manually evict the oldest entry. Returns the evicted key, if any.
    pub fn evict_one(&self) -> Option<String> {
        todo!()
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
        let store = BoundedStore::<Fractal<String>>::new(10);
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn insert_and_get() {
        let store = BoundedStore::<Fractal<String>>::new(10);
        let node = shard("hello");
        let key = oid(&node);
        store.insert(key.clone(), node.clone());
        assert_eq!(store.get(&key), Some(node));
    }

    #[test]
    fn respects_capacity() {
        let store = BoundedStore::<Fractal<String>>::new(2);
        for i in 0..5 {
            let node = shard(&format!("item-{i}"));
            let key = oid(&node);
            store.insert(key, node);
        }
        assert!(store.len() <= 2);
    }

    #[test]
    fn evicts_oldest() {
        let store = BoundedStore::<Fractal<String>>::new(2);
        let a = shard("aaa");
        let b = shard("bbb");
        let c = shard("ccc");
        let ka = oid(&a);
        let kb = oid(&b);
        let kc = oid(&c);

        store.insert(ka.clone(), a);
        store.insert(kb.clone(), b.clone());
        store.insert(kc.clone(), c.clone());

        // "aaa" was oldest — should be evicted.
        assert!(store.get(&ka).is_none(), "oldest entry should be evicted");
        // "bbb" and "ccc" should still be present.
        assert_eq!(store.get(&kb), Some(b));
        assert_eq!(store.get(&kc), Some(c));
    }

    #[test]
    fn get_missing_returns_none() {
        let store = BoundedStore::<Fractal<String>>::new(10);
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn evict_one_returns_key() {
        let store = BoundedStore::<Fractal<String>>::new(10);
        let node = shard("evict-me");
        let key = oid(&node);
        store.insert(key.clone(), node);
        let evicted = store.evict_one();
        assert_eq!(evicted, Some(key.clone()));
        assert!(store.get(&key).is_none());
    }
}
