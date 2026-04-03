//! `.frgmnt` content store — no git dependency. Files on disk.
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
use crate::encoding::{Decode, Encode};
use crate::fragment::{Fractal, Fragmentable};

/// Error type for FgmntStore operations.
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

/// Estimate byte size of a Fractal for cache accounting.
fn estimate_size<E: Encode>(fractal: &Fractal<E>) -> usize {
    let base = std::mem::size_of_val(fractal);
    let data_size = std::mem::size_of_val(fractal.data());
    base + data_size
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

/// A file-backed bounded store that writes to `.frgmnt/` on eviction
/// and reads from disk on cache miss. No git dependency.
pub struct FgmntStore<E: Encode + Decode + Clone> {
    cache: BoundedStore<Fractal<E>>,
    root: PathBuf,
    /// Protects flush — prevents concurrent flushes racing with inserts.
    _flush_lock: Mutex<()>,
}

impl<E: Encode + Decode + Clone> FgmntStore<E> {
    /// Open or create a `.frgmnt` store at the given path.
    /// Creates `.frgmnt/objects/` and `.frgmnt/refs/` if they don't exist.
    pub fn open(path: &str, max_bytes: usize) -> Result<Self, Error> {
        let root = PathBuf::from(path);
        std::fs::create_dir_all(root.join("objects"))?;
        std::fs::create_dir_all(root.join("refs"))?;
        Ok(FgmntStore {
            cache: BoundedStore::new(max_bytes),
            root,
            _flush_lock: Mutex::new(()),
        })
    }

    /// Write a Fractal to disk at its object path.
    fn write_to_disk(&self, oid: &str, value: &Fractal<E>) -> Result<(), Error> {
        let path = object_path(&self.root, oid);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = value.data().encode();
        std::fs::write(&path, &bytes)?;
        Ok(())
    }

    /// Read a Fractal from disk at its object path.
    fn read_from_disk(&self, oid: &str) -> Option<Fractal<E>> {
        let path = object_path(&self.root, oid);
        let bytes = std::fs::read(&path).ok()?;
        let data = E::decode(&bytes).ok()?;
        // Reconstruct as a shard (terminal fragment — we store data only).
        let ref_ = crate::ref_::Ref::new(
            crate::sha::Sha(oid.to_string()),
            "frgmnt",
        );
        Some(Fractal::Shard { ref_, data })
    }

    /// Insert a fractal. Stores in cache; on cache overflow the evicted entry
    /// is written to `.frgmnt/objects/` before being dropped.
    /// Content-addressed: if the key is already in cache, this is a no-op.
    pub fn insert(&self, key: String, value: Fractal<E>) {
        // Content-addressed dedup: skip if already cached.
        if self.cache.get(&key).is_some() {
            return;
        }
        let size = estimate_size(&value);
        // If inserting will cause eviction, persist the oldest entry first.
        if self.cache.total_bytes() + size > self.cache.capacity() {
            if let Some((oldest_key, oldest_node)) = self.cache.peek_oldest() {
                let _ = self.write_to_disk(&oldest_key, &oldest_node);
            }
        }
        self.cache.insert(key, value, size);
    }

    /// Get a Fractal by Oid. Checks cache first, falls back to disk on miss.
    pub fn get(&self, key: &str) -> Option<Fractal<E>> {
        // Hot path: in cache.
        if let Some(node) = self.cache.get(key) {
            return Some(node);
        }
        // Cold path: read from .frgmnt/objects/.
        let node = self.read_from_disk(key)?;
        let size = estimate_size(&node);
        self.cache.insert(key.to_string(), node.clone(), size);
        Some(node)
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

    /// Flush all cached entries to disk, then clear the cache.
    pub fn flush(&self) {
        let _guard = self._flush_lock.lock().unwrap();
        self.cache.drain_all(|key, node| {
            let _ = self.write_to_disk(key, node);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;
    use crate::fragment::{content_oid, Fragmentable};

    fn shard(label: &str) -> Fractal<String> {
        encoding::encode(label)
    }

    fn oid(node: &Fractal<String>) -> String {
        content_oid(node)
    }

    #[test]
    fn open_creates_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        FgmntStore::<String>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();
        assert!(frgmnt.join("objects").exists());
        assert!(frgmnt.join("refs").exists());
    }

    #[test]
    fn insert_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FgmntStore::<String>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();

        let node = shard("hello");
        let key = oid(&node);
        store.insert(key.clone(), node.clone());

        let got = store.get(&key);
        assert!(got.is_some());
        assert_eq!(got.unwrap().data(), node.data());
    }

    #[test]
    fn eviction_persists_to_frgmnt() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        // Very small cache — 100 bytes forces eviction after first entry.
        let store = FgmntStore::<String>::open(frgmnt.to_str().unwrap(), 100).unwrap();

        let a = shard("persist-a");
        let b = shard("persist-b");
        let c = shard("persist-c");
        let ka = oid(&a);

        store.insert(ka.clone(), a);
        store.insert(oid(&b), b);
        store.insert(oid(&c), c);

        // "persist-a" should have been written to disk on eviction.
        let path = object_path(&store.root, &ka);
        assert!(path.exists(), "evicted object should be on disk: {:?}", path);
    }

    #[test]
    fn cache_miss_reads_from_frgmnt() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FgmntStore::<String>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();

        // Write a node directly to disk, bypassing cache.
        let node = shard("cold-data");
        let key = oid(&node);
        let path = object_path(&store.root, &key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, node.data().encode()).unwrap();

        // Cache should be empty.
        assert_eq!(store.cached_len(), 0);

        // Reading should load from disk and promote to cache.
        let got = store.get(&key);
        assert!(got.is_some(), "should read from disk on cache miss");
        assert_eq!(store.cached_len(), 1, "should promote to cache after miss");
    }

    #[test]
    fn set_ref_get_ref() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FgmntStore::<String>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();

        let node = shard("ref-target");
        let key = oid(&node);

        store.set_ref("boot", &key).unwrap();
        let got = store.get_ref("boot");
        assert_eq!(got.as_deref(), Some(key.as_str()));
    }

    #[test]
    fn content_addressed_dedup() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FgmntStore::<String>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();

        let node1 = shard("same-content");
        let node2 = shard("same-content");
        let key1 = oid(&node1);
        let key2 = oid(&node2);

        // Same content → same Oid.
        assert_eq!(key1, key2);

        store.insert(key1.clone(), node1);
        store.insert(key2.clone(), node2);

        // Should only have one cache entry (same key).
        assert_eq!(store.cached_len(), 1);
    }

    #[test]
    fn flush_writes_all() {
        let dir = tempfile::tempdir().unwrap();
        let frgmnt = dir.path().join(".frgmnt");
        let store = FgmntStore::<String>::open(frgmnt.to_str().unwrap(), 10_000).unwrap();

        let a = shard("flush-a");
        let b = shard("flush-b");
        let ka = oid(&a);
        let kb = oid(&b);

        store.insert(ka.clone(), a);
        store.insert(kb.clone(), b);
        assert_eq!(store.cached_len(), 2);

        store.flush();

        // Cache should be empty.
        assert_eq!(store.cached_len(), 0);
        // Both objects should be on disk.
        assert!(object_path(&store.root, &ka).exists());
        assert!(object_path(&store.root, &kb).exists());
    }
}
