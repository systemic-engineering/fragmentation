use crate::encoding::Encode;
use crate::ref_::Ref;

/// Raw bytes. The default data type for fragments.
/// String is a lens an actor applies.
pub type Blob = Vec<u8>;

/// A node in the possibility space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fragment<E = Blob> {
    /// Terminal: self-addressed, carries data, stops.
    Shard { ref_: Ref, data: E },
    /// Self-similar: self-addressed, carries data, contains fragments.
    Fractal {
        ref_: Ref,
        data: E,
        fragments: Vec<Fragment<E>>,
    },
}

impl Fragment<String> {
    /// Create a shard from string-like data. Terminal fragment.
    pub fn shard(ref_: Ref, data: impl Into<String>) -> Self {
        Fragment::Shard {
            ref_,
            data: data.into(),
        }
    }

    /// Create a fractal from string-like data. Self-similar, contains other fragments.
    pub fn fractal(ref_: Ref, data: impl Into<String>, fragments: Vec<Fragment<String>>) -> Self {
        Fragment::Fractal {
            ref_,
            data: data.into(),
            fragments,
        }
    }
}

impl<E> Fragment<E> {
    /// Create a shard with typed data. Terminal fragment.
    pub fn shard_typed(ref_: Ref, data: E) -> Self {
        Fragment::Shard { ref_, data }
    }

    /// Create a fractal with typed data. Self-similar, contains other fragments.
    pub fn fractal_typed(ref_: Ref, data: E, fragments: Vec<Fragment<E>>) -> Self {
        Fragment::Fractal {
            ref_,
            data,
            fragments,
        }
    }

    /// Get the ref (self-address) of a fragment.
    pub fn self_ref(&self) -> &Ref {
        match self {
            Fragment::Shard { ref_, .. } => ref_,
            Fragment::Fractal { ref_, .. } => ref_,
        }
    }

    /// Get the data from a fragment.
    pub fn data(&self) -> &E {
        match self {
            Fragment::Shard { data, .. } => data,
            Fragment::Fractal { data, .. } => data,
        }
    }

    /// Get child fragments. Shards have none.
    pub fn children(&self) -> &[Fragment<E>] {
        match self {
            Fragment::Shard { .. } => &[],
            Fragment::Fractal { fragments, .. } => fragments,
        }
    }

    /// Check if a fragment is a shard.
    pub fn is_shard(&self) -> bool {
        matches!(self, Fragment::Shard { .. })
    }

    /// Check if a fragment is a fractal (non-terminal).
    pub fn is_fractal(&self) -> bool {
        matches!(self, Fragment::Fractal { .. })
    }
}

/// Compute a git-compatible content OID for a fragment.
/// Shard -> blob OID, Fragment -> tree OID.
/// Witness metadata is NOT included -- same content = same OID.
pub fn content_oid<E: Encode>(frag: &Fragment<E>) -> String {
    match frag {
        Fragment::Shard { data, .. } => blob_oid_bytes(&data.encode()),
        Fragment::Fractal {
            data, fragments, ..
        } => tree_oid_bytes(&data.encode(), fragments),
    }
}

/// Compute the git blob OID for string data.
/// SHA-1("blob {len}\0{data}") -- matches `git hash-object --stdin`.
pub fn blob_oid(data: &str) -> String {
    blob_oid_bytes(data.as_bytes())
}

/// Compute the git blob OID for raw byte data.
/// SHA-1("blob {len}\0{data}") -- matches `git hash-object --stdin`.
pub fn blob_oid_bytes(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let header = format!("blob {}\0", data.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Compute the git tree OID for a fragment with data and children.
/// Builds the same binary tree object that git would, then SHA-1 hashes it.
pub fn tree_oid<E: Encode>(data: &str, children: &[Fragment<E>]) -> String {
    tree_oid_bytes(data.as_bytes(), children)
}

/// Compute the git tree OID for a fragment with byte data and children.
pub fn tree_oid_bytes<E: Encode>(data: &[u8], children: &[Fragment<E>]) -> String {
    use sha1::{Digest, Sha1};

    let tree_bytes = build_tree_bytes(data, children);
    let header = format!("tree {}\0", tree_bytes.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(&tree_bytes);
    hex::encode(hasher.finalize())
}

/// Build the raw bytes of a git tree object (without header).
/// Entries: ".data" blob + "0000", "0001", ... numbered children.
/// Each entry: "{mode} {name}\0{20-byte SHA-1}"
fn build_tree_bytes<E: Encode>(data: &[u8], children: &[Fragment<E>]) -> Vec<u8> {
    let mut entries: Vec<(String, u32, [u8; 20])> = Vec::new();

    // .data entry -- the fragment's own data as a blob
    let data_oid_hex = blob_oid_bytes(data);
    let data_oid_bytes = hex_to_bytes20(&data_oid_hex);
    entries.push((".data".to_string(), 0o100644, data_oid_bytes));

    // Numbered children
    for (i, child) in children.iter().enumerate() {
        let child_oid_hex = content_oid(child);
        let child_oid_bytes = hex_to_bytes20(&child_oid_hex);
        let mode = if child.is_shard() { 0o100644 } else { 0o040000 };
        entries.push((format!("{:04}", i), mode, child_oid_bytes));
    }

    // Git sorts tree entries by name (byte order)
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut buf = Vec::new();
    for (name, mode, oid) in &entries {
        buf.extend_from_slice(format!("{} {}\0", mode_to_string(*mode), name).as_bytes());
        buf.extend_from_slice(oid);
    }
    buf
}

/// Format mode as git does: no leading zeros for trees (40000), six digits for blobs (100644).
fn mode_to_string(mode: u32) -> String {
    if mode == 0o040000 {
        "40000".to_string()
    } else {
        format!("{:o}", mode)
    }
}

/// Convert a 40-char hex string to 20 raw bytes.
fn hex_to_bytes20(hex_str: &str) -> [u8; 20] {
    let bytes = hex::decode(hex_str).expect("valid hex");
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes);
    arr
}
