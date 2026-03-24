//! NakedSingularity — self-contained artifact. Observer in content. No repo required.
//!
//! A NakedSingularity carries its content, witness metadata, and dual OIDs:
//! - `content_oid`: hash of tree content only. Observer-independent.
//! - `naked_oid`: hash of tree content + witness. Observer-dependent.
//!
//! The cosmic censorship violation: the observer is in the content hash.

use crate::cid::{Cid, Codec, HashId};
use crate::encoding::Encode;
use crate::fragment::{content_oid, Fractal};
use crate::ref_::Ref;
use crate::sha::{HashAlg, Sha};
use crate::singularity::Singularity;
use crate::witnessed::Witnessed;

/// A self-contained artifact. No repo interaction needed.
///
/// Dual OID semantics:
/// - `content_oid`: hash of tree content only. Same tree = same content_oid
///   regardless of who collapses it.
/// - `naked_oid`: hash of tree content + witness metadata. Same tree +
///   different witness = different naked_oid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NakedSingularity<E: Clone + Encode, H: HashAlg = Sha> {
    content: Fractal<E, H>,
    witness: Witnessed,
    content_cid: Cid<H>,
    naked_cid: Cid<H>,
}

/// Error type for naked singularity operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NakedError {
    /// Failed to serialize the artifact.
    SerializationError(String),
    /// Failed to deserialize the artifact.
    DeserializationError(String),
}

impl std::fmt::Display for NakedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NakedError::SerializationError(msg) => write!(f, "serialization error: {msg}"),
            NakedError::DeserializationError(msg) => write!(f, "deserialization error: {msg}"),
        }
    }
}

/// Self-contained byte bundle. The collapsed form of a NakedSingularity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NakedArtifact<H: HashAlg = Sha> {
    bytes: Vec<u8>,
    content_cid: Cid<H>,
    naked_cid: Cid<H>,
}

impl<E: Clone + Encode, H: HashAlg> NakedSingularity<E, H> {
    /// Construct a NakedSingularity from a Fractal and witness metadata.
    /// Computes both content_oid (observer-independent) and naked_oid (observer-dependent).
    pub fn new(content: Fractal<E, H>, witness: Witnessed) -> Self {
        todo!()
    }

    /// The content (tree).
    pub fn content(&self) -> &Fractal<E, H> {
        todo!()
    }

    /// The witness metadata.
    pub fn witness(&self) -> &Witnessed {
        todo!()
    }

    /// The content CID (observer-independent).
    pub fn content_cid(&self) -> &Cid<H> {
        todo!()
    }

    /// The naked CID (observer-dependent).
    pub fn naked_cid(&self) -> &Cid<H> {
        todo!()
    }
}

impl<H: HashAlg> NakedArtifact<H> {
    /// The serialized bytes.
    pub fn bytes(&self) -> &[u8] {
        todo!()
    }

    /// The content CID.
    pub fn content_cid(&self) -> &Cid<H> {
        todo!()
    }

    /// The naked CID.
    pub fn naked_cid(&self) -> &Cid<H> {
        todo!()
    }
}

impl<E: Clone + Encode, H: HashAlg> Singularity for NakedSingularity<E, H> {
    type Artifact = NakedArtifact<H>;
    type Error = NakedError;

    fn collapse(&self) -> Result<Self::Artifact, Self::Error> {
        todo!()
    }

    fn refract(_artifact: &Self::Artifact) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;
    use crate::fragment::Fractal;
    use crate::sha::Sha;
    use crate::witnessed::{Author, Committer, Timestamp, Witnessed};

    fn test_tree() -> Fractal<String> {
        encoding::encode("the observer is part of the hash")
    }

    fn mara_witness() -> Witnessed {
        Witnessed::new(
            Author::new("Mara", "mara@systemic.engineer"),
            Committer::new("Mara", "mara@systemic.engineer"),
            Timestamp("1234567890 +0000".into()),
        )
    }

    fn reed_witness() -> Witnessed {
        Witnessed::new(
            Author::new("Reed", "reed@systemic.engineer"),
            Committer::new("Reed", "reed@systemic.engineer"),
            Timestamp("1234567890 +0000".into()),
        )
    }

    // -- construction --

    #[test]
    fn construction_from_fractal_and_witness() {
        let tree = test_tree();
        let witness = mara_witness();
        let naked = NakedSingularity::new(tree.clone(), witness.clone());
        assert_eq!(naked.content(), &tree);
        assert_eq!(naked.witness(), &witness);
    }

    // -- dual OID --

    #[test]
    fn same_content_same_witness_same_naked_oid() {
        let tree = test_tree();
        let witness = mara_witness();
        let a = NakedSingularity::new(tree.clone(), witness.clone());
        let b = NakedSingularity::new(tree, witness);
        assert_eq!(a.naked_cid(), b.naked_cid());
    }

    #[test]
    fn same_content_different_witness_same_content_oid() {
        let tree = test_tree();
        let a = NakedSingularity::new(tree.clone(), mara_witness());
        let b = NakedSingularity::new(tree, reed_witness());
        assert_eq!(a.content_cid(), b.content_cid());
    }

    #[test]
    fn same_content_different_witness_different_naked_oid() {
        let tree = test_tree();
        let a = NakedSingularity::new(tree.clone(), mara_witness());
        let b = NakedSingularity::new(tree, reed_witness());
        assert_ne!(a.naked_cid(), b.naked_cid());
    }

    #[test]
    fn different_content_same_witness_different_content_oid() {
        let tree1 = encoding::encode("first content");
        let tree2 = encoding::encode("second content");
        let witness = mara_witness();
        let a = NakedSingularity::new(tree1, witness.clone());
        let b = NakedSingularity::new(tree2, witness);
        assert_ne!(a.content_cid(), b.content_cid());
    }

    // -- singularity trait --

    #[test]
    fn collapse_produces_artifact() {
        let tree = test_tree();
        let naked = NakedSingularity::new(tree, mara_witness());
        let artifact = naked.collapse();
        assert!(artifact.is_ok());
    }

    #[test]
    fn collapse_preserves_cids() {
        let tree = test_tree();
        let naked = NakedSingularity::new(tree, mara_witness());
        let artifact = naked.collapse().unwrap();
        assert_eq!(artifact.content_cid(), naked.content_cid());
        assert_eq!(artifact.naked_cid(), naked.naked_cid());
    }

    #[test]
    fn refract_recovers_original() {
        let tree = test_tree();
        let naked = NakedSingularity::new(tree, mara_witness());
        let artifact = naked.collapse().unwrap();
        let recovered = NakedSingularity::<String>::refract(&artifact).unwrap();
        assert_eq!(recovered.content(), naked.content());
        assert_eq!(recovered.witness(), naked.witness());
    }

    #[test]
    fn round_trip_preserves_both_cids() {
        let tree = test_tree();
        let naked = NakedSingularity::new(tree, mara_witness());
        let artifact = naked.collapse().unwrap();
        let recovered = NakedSingularity::<String>::refract(&artifact).unwrap();
        assert_eq!(recovered.content_cid(), naked.content_cid());
        assert_eq!(recovered.naked_cid(), naked.naked_cid());
    }

    #[test]
    fn artifact_bytes_not_empty() {
        let tree = test_tree();
        let naked = NakedSingularity::new(tree, mara_witness());
        let artifact = naked.collapse().unwrap();
        assert!(!artifact.bytes().is_empty());
    }

    #[test]
    fn artifact_bytes_deterministic() {
        let tree = test_tree();
        let a = NakedSingularity::new(tree.clone(), mara_witness())
            .collapse()
            .unwrap();
        let b = NakedSingularity::new(tree, mara_witness())
            .collapse()
            .unwrap();
        assert_eq!(a.bytes(), b.bytes());
    }
}
