//! Self-describing content identifier.
//!
//! CID-like, not IPLD-compatible. Our own format.
//! Wraps `Ref<H>` with self-describing metadata: codec and hash algorithm.
//! `Ref<H>` stays unchanged — `Cid<H>` adds the self-describing envelope.

use std::fmt;

use crate::ref_::Ref;
use crate::sha::{HashAlg, Sha};

/// Hash algorithm identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HashId {
    Sha256,
}

/// Data codec identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Codec {
    /// Our native serialization (fragmentation encoding).
    Fragmentation,
}

/// Self-describing content identifier.
///
/// Wraps a `Ref<H>` with codec and hash algorithm metadata.
/// Zero blast radius on existing code — `Ref<H>` is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cid<H: HashAlg = Sha> {
    pub ref_: Ref<H>,
    pub codec: Codec,
    pub hash_id: HashId,
}

impl<H: HashAlg> Cid<H> {
    /// Construct a new CID from a Ref, codec, and hash identifier.
    pub fn new(ref_: Ref<H>, codec: Codec, hash_id: HashId) -> Self {
        Cid {
            ref_,
            codec,
            hash_id,
        }
    }

    /// Construct a CID with default codec (Fragmentation) and hash (Sha256).
    pub fn from_ref(ref_: Ref<H>) -> Self {
        Cid {
            ref_,
            codec: Codec::Fragmentation,
            hash_id: HashId::Sha256,
        }
    }

    /// The underlying reference.
    pub fn ref_(&self) -> &Ref<H> {
        &self.ref_
    }

    /// The hash as a hex string.
    pub fn hash_hex(&self) -> &str {
        self.ref_.sha.as_str()
    }

    /// The label from the underlying Ref.
    pub fn label(&self) -> &str {
        &self.ref_.label
    }
}

impl<H: HashAlg> fmt::Display for Cid<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ref_.sha.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha::Sha;

    fn test_ref() -> Ref<Sha> {
        Ref::new(Sha::hash(b"test content"), "test-label")
    }

    #[test]
    fn cid_construction() {
        let r = test_ref();
        let cid = Cid::new(r.clone(), Codec::Fragmentation, HashId::Sha256);
        assert_eq!(cid.ref_(), &r);
        assert_eq!(cid.codec, Codec::Fragmentation);
        assert_eq!(cid.hash_id, HashId::Sha256);
    }

    #[test]
    fn cid_from_ref_defaults() {
        let r = test_ref();
        let cid = Cid::from_ref(r.clone());
        assert_eq!(cid.ref_(), &r);
        assert_eq!(cid.codec, Codec::Fragmentation);
        assert_eq!(cid.hash_id, HashId::Sha256);
    }

    #[test]
    fn cid_hash_hex() {
        let r = test_ref();
        let cid = Cid::from_ref(r.clone());
        assert_eq!(cid.hash_hex(), r.sha.as_str());
    }

    #[test]
    fn cid_label() {
        let r = test_ref();
        let cid = Cid::from_ref(r);
        assert_eq!(cid.label(), "test-label");
    }

    #[test]
    fn cid_display() {
        let r = test_ref();
        let cid = Cid::from_ref(r.clone());
        let display = cid.to_string();
        // Display should include the hash hex
        assert!(display.contains(r.sha.as_str()));
    }

    #[test]
    fn cid_equality() {
        let r = test_ref();
        let a = Cid::from_ref(r.clone());
        let b = Cid::from_ref(r);
        assert_eq!(a, b);
    }

    #[test]
    fn cid_different_refs_differ() {
        let r1 = Ref::new(Sha::hash(b"content-a"), "label-a");
        let r2 = Ref::new(Sha::hash(b"content-b"), "label-b");
        let a = Cid::from_ref(r1);
        let b = Cid::from_ref(r2);
        assert_ne!(a, b);
    }
}
