//! NakedSingularity — self-contained artifact. Observer in content. No repo required.
//!
//! A NakedSingularity carries its content, witness metadata, and dual OIDs:
//! - `content_oid`: hash of tree content only. Observer-independent.
//! - `naked_oid`: hash of tree content + witness. Observer-dependent.
//!
//! The cosmic censorship violation: the observer is in the content hash.

use crate::cid::Cid;
use crate::encoding::{Decode, Encode};
use crate::fragment::{content_oid, Fractal};
use crate::ref_::Ref;
use crate::sha::{HashAlg, Sha};
use crate::singularity::Singularity;
use crate::witnessed::{Author, Committer, Timestamp, Witnessed};

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

impl std::error::Error for NakedError {}

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
        let content_oid_hex = content_oid(&content);
        let content_cid = Cid::from_ref(Ref::new(H::from_hex(&content_oid_hex), "content"));

        // naked_oid = hash(content_oid + witness metadata)
        let witness_bytes = serialize_witnessed(&witness);
        let mut naked_input = content_oid_hex.as_bytes().to_vec();
        naked_input.extend_from_slice(b":");
        naked_input.extend_from_slice(&witness_bytes);
        let naked_hash = <H as HashAlg>::hash(&naked_input);
        let naked_cid = Cid::from_ref(Ref::new(naked_hash, "naked"));

        NakedSingularity {
            content,
            witness,
            content_cid,
            naked_cid,
        }
    }

    /// The content (tree).
    pub fn content(&self) -> &Fractal<E, H> {
        &self.content
    }

    /// The witness metadata.
    pub fn witness(&self) -> &Witnessed {
        &self.witness
    }

    /// The content CID (observer-independent).
    pub fn content_cid(&self) -> &Cid<H> {
        &self.content_cid
    }

    /// The naked CID (observer-dependent).
    pub fn naked_cid(&self) -> &Cid<H> {
        &self.naked_cid
    }
}

impl<H: HashAlg> NakedArtifact<H> {
    /// The serialized bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The content CID.
    pub fn content_cid(&self) -> &Cid<H> {
        &self.content_cid
    }

    /// The naked CID.
    pub fn naked_cid(&self) -> &Cid<H> {
        &self.naked_cid
    }
}

impl<E: Clone + Encode + Decode, H: HashAlg> Singularity for NakedSingularity<E, H> {
    type Artifact = NakedArtifact<H>;
    type Error = NakedError;

    fn collapse(&self) -> Result<Self::Artifact, Self::Error> {
        let mut bytes = Vec::new();
        serialize_fractal(&self.content, &mut bytes);
        bytes.extend_from_slice(b"|");
        bytes.extend_from_slice(&serialize_witnessed(&self.witness));

        Ok(NakedArtifact {
            bytes,
            content_cid: self.content_cid.clone(),
            naked_cid: self.naked_cid.clone(),
        })
    }

    fn refract(artifact: &Self::Artifact) -> Result<Self, Self::Error> {
        let bytes = &artifact.bytes;

        // Find the separator between fractal and witness
        let sep = find_separator(bytes)
            .ok_or_else(|| NakedError::DeserializationError("missing separator".into()))?;

        let fractal_bytes = &bytes[..sep];
        let witness_bytes = &bytes[sep + 1..];

        let mut cursor = 0;
        let content: Fractal<E, H> = deserialize_fractal(fractal_bytes, &mut cursor)
            .ok_or_else(|| NakedError::DeserializationError("failed to deserialize fractal".into()))?;

        let witness = deserialize_witnessed(witness_bytes)
            .ok_or_else(|| NakedError::DeserializationError("failed to deserialize witness".into()))?;

        Ok(NakedSingularity::new(content, witness))
    }
}

// ============================================================================
// Serialization helpers
// ============================================================================

fn write_u32(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    if *cursor + 4 > bytes.len() {
        return None;
    }
    let val = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().ok()?);
    *cursor += 4;
    Some(val)
}

fn write_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    write_u32(buf, data.len() as u32);
    buf.extend_from_slice(data);
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    let len = read_u32(bytes, cursor)? as usize;
    if *cursor + len > bytes.len() {
        return None;
    }
    let data = &bytes[*cursor..*cursor + len];
    *cursor += len;
    Some(data)
}

fn serialize_fractal<E: Encode, H: HashAlg>(frac: &Fractal<E, H>, buf: &mut Vec<u8>) {
    match frac {
        Fractal::Shard { ref_, data } => {
            buf.push(0); // tag
            write_bytes(buf, ref_.sha.as_str().as_bytes());
            write_bytes(buf, ref_.label.as_bytes());
            write_bytes(buf, &data.encode());
        }
        Fractal::Fractal {
            ref_,
            data,
            fractal,
        } => {
            buf.push(1); // tag
            write_bytes(buf, ref_.sha.as_str().as_bytes());
            write_bytes(buf, ref_.label.as_bytes());
            write_bytes(buf, &data.encode());
            write_u32(buf, fractal.len() as u32);
            for child in fractal {
                serialize_fractal(child, buf);
            }
        }
        Fractal::Lens {
            ref_,
            data,
            target,
        } => {
            buf.push(2); // tag
            write_bytes(buf, ref_.sha.as_str().as_bytes());
            write_bytes(buf, ref_.label.as_bytes());
            write_bytes(buf, &data.encode());
            write_u32(buf, target.len() as u32);
            for t in target {
                write_bytes(buf, t.as_str().as_bytes());
            }
        }
    }
}

fn deserialize_fractal<E: Decode, H: HashAlg>(
    bytes: &[u8],
    cursor: &mut usize,
) -> Option<Fractal<E, H>> {
    if *cursor >= bytes.len() {
        return None;
    }
    let tag = bytes[*cursor];
    *cursor += 1;

    let sha_bytes = read_bytes(bytes, cursor)?;
    let sha_str = std::str::from_utf8(sha_bytes).ok()?;
    let sha = H::from_hex(sha_str);

    let label_bytes = read_bytes(bytes, cursor)?;
    let label = std::str::from_utf8(label_bytes).ok()?.to_string();

    let data_bytes = read_bytes(bytes, cursor)?;
    let data = E::decode(data_bytes).ok()?;

    let ref_ = Ref::new(sha, label);

    match tag {
        0 => Some(Fractal::Shard { ref_, data }),
        1 => {
            let count = read_u32(bytes, cursor)? as usize;
            let remaining = bytes.len().saturating_sub(*cursor);
            if count > remaining {
                return None;
            }
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                children.push(deserialize_fractal(bytes, cursor)?);
            }
            Some(Fractal::Fractal {
                ref_,
                data,
                fractal: children,
            })
        }
        2 => {
            let count = read_u32(bytes, cursor)? as usize;
            let remaining = bytes.len().saturating_sub(*cursor);
            if count > remaining {
                return None;
            }
            let mut targets = Vec::with_capacity(count);
            for _ in 0..count {
                let t_bytes = read_bytes(bytes, cursor)?;
                let t_str = std::str::from_utf8(t_bytes).ok()?;
                targets.push(H::from_hex(t_str));
            }
            Some(Fractal::Lens {
                ref_,
                data,
                target: targets,
            })
        }
        _ => None,
    }
}

fn serialize_witnessed(w: &Witnessed) -> Vec<u8> {
    let mut buf = Vec::new();
    write_bytes(&mut buf, w.author.name.as_bytes());
    write_bytes(&mut buf, w.author.email.as_bytes());
    write_bytes(&mut buf, w.committer.name.as_bytes());
    write_bytes(&mut buf, w.committer.email.as_bytes());
    write_bytes(&mut buf, w.timestamp.0.as_bytes());
    buf
}

fn deserialize_witnessed(bytes: &[u8]) -> Option<Witnessed> {
    let mut cursor = 0;
    let author_name = std::str::from_utf8(read_bytes(bytes, &mut cursor)?).ok()?;
    let author_email = std::str::from_utf8(read_bytes(bytes, &mut cursor)?).ok()?;
    let committer_name = std::str::from_utf8(read_bytes(bytes, &mut cursor)?).ok()?;
    let committer_email = std::str::from_utf8(read_bytes(bytes, &mut cursor)?).ok()?;
    let timestamp = std::str::from_utf8(read_bytes(bytes, &mut cursor)?).ok()?;
    Some(Witnessed::new(
        Author::new(author_name, author_email),
        Committer::new(committer_name, committer_email),
        Timestamp(timestamp.into()),
    ))
}

/// Find the `|` separator between fractal bytes and witness bytes.
/// Walks the fractal structure to find where it ends, then expects `|`.
fn find_separator(bytes: &[u8]) -> Option<usize> {
    let mut cursor = 0;
    skip_fractal(bytes, &mut cursor)?;
    if cursor < bytes.len() && bytes[cursor] == b'|' {
        Some(cursor)
    } else {
        None
    }
}

/// Skip past a serialized fractal without allocating, advancing the cursor.
fn skip_fractal(bytes: &[u8], cursor: &mut usize) -> Option<()> {
    if *cursor >= bytes.len() {
        return None;
    }
    let tag = bytes[*cursor];
    *cursor += 1;

    // sha
    let len = read_u32(bytes, cursor)? as usize;
    *cursor += len;
    // label
    let len = read_u32(bytes, cursor)? as usize;
    *cursor += len;
    // data
    let len = read_u32(bytes, cursor)? as usize;
    *cursor += len;

    match tag {
        0 => Some(()),
        1 => {
            let count = read_u32(bytes, cursor)? as usize;
            for _ in 0..count {
                skip_fractal(bytes, cursor)?;
            }
            Some(())
        }
        2 => {
            let count = read_u32(bytes, cursor)? as usize;
            for _ in 0..count {
                let len = read_u32(bytes, cursor)? as usize;
                *cursor += len;
            }
            Some(())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;
    use crate::fragment::Fractal;
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
