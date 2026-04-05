//! Size-bounded concurrent store with LIFO eviction.
//!
//! [`BoundedStore`]: in-memory cache with capacity limit. Eviction drops.
//! For git-backed persistence, see the `fragmentation-git` crate.

use std::collections::VecDeque;
use std::sync::Mutex;

use dashmap::DashMap;

/// A byte-bounded concurrent content-addressed store.
///
/// Uses a `DashMap` for O(1) concurrent lookup and a `VecDeque` to track
/// insertion order. On insert, if total bytes exceed `max_bytes`, the oldest
/// entries (back of the deque) are evicted until the store fits.
pub struct BoundedStore<N: Clone> {
    objects: DashMap<String, N>,
    sizes: DashMap<String, usize>,
    order: Mutex<VecDeque<String>>,
    total_bytes: Mutex<usize>,
    max_bytes: usize,
}

impl<N: Clone> BoundedStore<N> {
    /// Create a new bounded store with the given byte capacity.
    pub fn new(max_bytes: usize) -> Self {
        BoundedStore {
            objects: DashMap::new(),
            sizes: DashMap::new(),
            order: Mutex::new(VecDeque::new()),
            total_bytes: Mutex::new(0),
            max_bytes,
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

