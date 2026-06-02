//! `.frgmnt` content store — no git dependency. Files on disk.
//!
//! Two modes based on trait bounds:
//! - **In-memory** (`N: Fragmentable + Clone`): bounded cache, eviction drops.
//! - **Persistent** (`N: Reconstructable + Clone`): bounded cache + `.frgmnt/`
//!   disk spillover. Evicted entries persist. Cache misses fall back to disk.
//!
//! Structure:
//! ```text
//! .frgmnt/
//!   objects/  — content by Oid, fan-out by first 2 hex chars
//!   refs/     — named pointers (plain text files containing an Oid)
//! ```

use std::path::PathBuf;
use std::sync::Mutex;

use crate::bounded_store::BoundedStore;
use crate::encoding::Encode;
use crate::fragment::{ContentAddressed, Fragmentable, Reconstructable, TreeShaped};

/// Error type for FrgmntStore operations.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Decode(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {}", e),
            Error::Decode(s) => write!(f, "decode: {}", s),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Return the object path for a given oid inside a .frgmnt root.
/// Fan-out: `.frgmnt/objects/<first-2>/<rest>`.
fn object_path(root: &PathBuf, oid: &str) -> PathBuf {
    if oid.len() < 2 {
        return root.join("objects").join(oid);
    }
    let (prefix, rest) = oid.split_at(2);
    root.join("objects").join(prefix).join(rest)
}

/// A bounded content-addressed store. In-memory cache with optional
/// `.frgmnt/` disk persistence for types that implement Reconstructable.
pub struct FrgmntStore<N: Fragmentable + Clone> {
    cache: BoundedStore<N>,
    root: PathBuf,
    _flush_lock: Mutex<()>,
}

// ---------------------------------------------------------------------------
// Core — available for any N: Fragmentable + Clone
// ---------------------------------------------------------------------------

impl<N: Fragmentable + Clone> FrgmntStore<N> {
    /// Open or create a `.frgmnt` store at the given path.
    pub fn open(path: &str, max_bytes: usize) -> Result<Self, Error> {
        let root = PathBuf::from(path);
        std::fs::create_dir_all(root.join("objects"))?;
        std::fs::create_dir_all(root.join("refs"))?;
        Ok(FrgmntStore {
            cache: BoundedStore::new(max_bytes),
            root,
            _flush_lock: Mutex::new(()),
        })
    }

    /// Insert a fragment by key with its byte size. Eviction drops.
    /// Content-addressed: if the key is already cached, this is a no-op.
    pub fn insert(&self, key: String, value: N, size_bytes: usize) {
        if self.cache.get(&key).is_some() {
            return;
        }
        self.cache.insert(key, value, size_bytes);
    }

    /// Get a fragment by key. Cache only — no disk fallback.
    pub fn get(&self, key: &str) -> Option<N> {
        self.cache.get(key)
    }

    /// Write a named ref (e.g. "boot" → Oid).
    pub fn set_ref(&self, name: &str, oid: &str) -> Result<(), Error> {
        let path = self.root.join("refs").join(name);
        std::fs::write(&path, oid)?;
        Ok(())
    }

    /// Read a named ref.
    pub fn get_ref(&self, name: &str) -> Option<String> {
        let path = self.root.join("refs").join(name);
        std::fs::read_to_string(&path).ok()
    }

    /// Number of cached entries.
    pub fn cached_len(&self) -> usize {
        self.cache.len()
    }

    /// Total bytes in cache.
    pub fn total_bytes(&self) -> usize {
        self.cache.total_bytes()
    }

    /// Maximum bytes this store will hold.
    pub fn capacity(&self) -> usize {
        self.cache.capacity()
    }
}

// ---------------------------------------------------------------------------
// Persistent — disk spillover for types that can reconstruct from bytes
// ---------------------------------------------------------------------------

impl<N: Reconstructable + Clone> FrgmntStore<N>
where
    N::Data: Encode + crate::encoding::Decode,
    N::Hash: crate::sha::HashAlg,
{
    /// Write a fragment to disk at its object path.
    fn write_to_disk(&self, oid: &str, value: &N) -> Result<(), Error> {
        let path = object_path(&self.root, oid);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = value.data().encode();
        std::fs::write(&path, &bytes)?;
        Ok(())
    }

    /// Read a fragment from disk at its object path.
    fn read_from_disk(&self, oid: &str) -> Option<N> {
        let path = object_path(&self.root, oid);
        let bytes = std::fs::read(&path).ok()?;
        let data = <N::Data as crate::encoding::Decode>::decode(&bytes).ok()?;
        use crate::sha::HashAlg as _;
        let ref_ = crate::ref_::Ref::new(N::Hash::from_hex(oid), "frgmnt");
        Some(N::reconstruct(ref_, data, vec![]))
    }

    /// Insert with disk persistence. On cache overflow, the evicted entry
    /// is written to `.frgmnt/objects/` before being dropped.
    pub fn insert_persistent(&self, key: String, value: N, size_bytes: usize) {
        if self.cache.get(&key).is_some() {
            return;
        }
        if self.cache.total_bytes() + size_bytes > self.cache.capacity() {
            if let Some((oldest_key, oldest_node)) = self.cache.peek_oldest() {
                let _ = self.write_to_disk(&oldest_key, &oldest_node);
            }
        }
        self.cache.insert(key, value, size_bytes);
    }

    /// Get with disk fallback. Checks cache first, reads from disk on miss,
    /// promotes to cache on hit.
    pub fn get_persistent(&self, key: &str) -> Option<N> {
        if let Some(node) = self.cache.get(key) {
            return Some(node);
        }
        let node = self.read_from_disk(key)?;
        let size = std::mem::size_of_val(&node);
        self.cache.insert(key.to_string(), node.clone(), size);
        Some(node)
    }

    /// Flush all cached entries to disk, then clear the cache.
    pub fn flush(&self) {
        let _guard = self._flush_lock.lock().unwrap();
        self.cache.drain_all(|key, node| {
            let _ = self.write_to_disk(key, node);
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;
    use crate::fragment::{self, Fractal};

    fn shard(label: &str) -> Fractal<String> {
        encoding::encode(label)
    }

    fn oid(node: &Fractal<String>) -> String {
        fragment::content_oid(node)
    }

    // -- Core tests (Fragmentable + Clone) --

    #[test]
    fn open_creates_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();
        assert!(frgmnt.join("objects").exists());
        assert!(frgmnt.join("refs").exists());
    }

    #[test]
    fn insert_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();
        let node = shard("hello");
        let key = oid(&node);
        store.insert(key.clone(), node.clone(), 100);
        let got = store.get(&key);
        assert!(got.is_some());
        assert_eq!(got.unwrap().data(), node.data());
    }

    #[test]
    fn eviction_drops_without_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 100).unwrap();
        let a = shard("aaa");
        let b = shard("bbb");
        let c = shard("ccc");
        let ka = oid(&a);
        store.insert(ka.clone(), a, 50);
        store.insert(oid(&b), b, 50);
        store.insert(oid(&c), c, 50);
        // "aaa" evicted from cache, not written to disk (used insert, not insert_persistent)
        assert!(store.get(&ka).is_none());
    }

    #[test]
    fn content_addressed_dedup() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();
        let n1 = shard("same");
        let n2 = shard("same");
        let k1 = oid(&n1);
        let k2 = oid(&n2);
        assert_eq!(k1, k2);
        store.insert(k1, n1, 50);
        store.insert(k2, n2, 50);
        assert_eq!(store.cached_len(), 1);
    }

    #[test]
    fn set_ref_get_ref() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();
        store.set_ref("boot", "abc123").unwrap();
        assert_eq!(store.get_ref("boot").as_deref(), Some("abc123"));
    }

    // -- Persistent tests (Reconstructable + Clone) --

    #[test]
    fn insert_persistent_evicts_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 100).unwrap();
        let a = shard("persist-a");
        let b = shard("persist-b");
        let c = shard("persist-c");
        let ka = oid(&a);
        store.insert_persistent(ka.clone(), a, 50);
        store.insert_persistent(oid(&b), b, 50);
        store.insert_persistent(oid(&c), c, 50);
        let path = object_path(&store.root, &ka);
        assert!(
            path.exists(),
            "evicted object should be on disk: {:?}",
            path
        );
    }

    #[test]
    fn get_persistent_falls_back_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();
        // Write directly to disk, bypassing cache.
        let path = object_path(&store.root, "cold");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "cold-data".as_bytes()).unwrap();
        assert_eq!(store.cached_len(), 0);
        let got = store.get_persistent("cold");
        assert!(got.is_some());
        assert_eq!(got.unwrap().data(), "cold-data");
        assert_eq!(store.cached_len(), 1);
    }

    #[test]
    fn flush_writes_all() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FrgmntStore::<Fractal<String>>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();
        let a = shard("flush-a");
        let b = shard("flush-b");
        let ka = oid(&a);
        let kb = oid(&b);
        store.insert(ka.clone(), a, 50);
        store.insert(kb.clone(), b, 50);
        assert_eq!(store.cached_len(), 2);
        store.flush();
        assert_eq!(store.cached_len(), 0);
        assert!(object_path(&store.root, &ka).exists());
        assert!(object_path(&store.root, &kb).exists());
    }
}
