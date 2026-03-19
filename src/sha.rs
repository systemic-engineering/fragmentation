use std::fmt;
use std::hash;

use sha2::{Digest, Sha256};

/// Trait for pluggable hash algorithms.
///
/// Downstream crates can implement this for their own hash types.
/// The default throughout fragmentation is [`Sha`] (SHA-256).
pub trait HashAlg: Clone + fmt::Debug + PartialEq + Eq + hash::Hash {
    /// Hash raw bytes, returning a new instance of this hash type.
    fn hash(data: &[u8]) -> Self;
    /// Construct from an existing hex string (e.g., a git commit SHA).
    fn from_hex(hex: impl Into<String>) -> Self;
    /// View the hash as a hex string.
    fn as_str(&self) -> &str;
}

/// Content-addressed hash (SHA-256).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sha(pub String);

impl HashAlg for Sha {
    fn hash(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Sha(hex::encode(hasher.finalize()))
    }

    fn from_hex(hex: impl Into<String>) -> Self {
        Sha(hex.into())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Raw SHA-256 hash of a string. Convenience wrapper around `HashAlg::hash`.
pub fn hash(data: &str) -> Sha {
    Sha::hash(data.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha_implements_hash_alg() {
        let h = Sha::hash(b"hello");
        assert!(!h.as_str().is_empty());
        // SHA-256 produces 64 hex chars
        assert_eq!(h.as_str().len(), 64);
    }

    #[test]
    fn hash_alg_deterministic() {
        let a = Sha::hash(b"test");
        let b = Sha::hash(b"test");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_alg_different_input_different_output() {
        let a = Sha::hash(b"hello");
        let b = Sha::hash(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_alg_as_str_matches_inner() {
        let h = Sha::hash(b"test");
        assert_eq!(h.as_str(), h.0.as_str());
    }

    #[test]
    fn hash_convenience_still_works() {
        // The old `hash()` function still works
        let h = hash("hello");
        assert_eq!(h.as_str().len(), 64);
    }
}
