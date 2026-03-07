use crate::ref_::{self, Ref};
use crate::sha;
use crate::witnessed::{self, Witnessed};

/// A node in the possibility space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fragment {
    /// Terminal: self-addressed, witnessed, carries data, stops.
    Shard {
        ref_: Ref,
        witnessed: Witnessed,
        data: String,
    },
    /// Self-similar: self-addressed, witnessed, carries data, contains fragments.
    Fragment {
        ref_: Ref,
        witnessed: Witnessed,
        data: String,
        fragments: Vec<Fragment>,
    },
}

impl Fragment {
    /// Create a shard. Terminal fragment.
    pub fn shard(ref_: Ref, witnessed: Witnessed, data: impl Into<String>) -> Self {
        Fragment::Shard {
            ref_,
            witnessed,
            data: data.into(),
        }
    }

    /// Create a fragment. Self-similar, contains other fragments.
    pub fn new_fragment(
        ref_: Ref,
        witnessed: Witnessed,
        data: impl Into<String>,
        fragments: Vec<Fragment>,
    ) -> Self {
        Fragment::Fragment {
            ref_,
            witnessed,
            data: data.into(),
            fragments,
        }
    }

    /// Get the ref (self-address) of a fragment.
    pub fn self_ref(&self) -> &Ref {
        match self {
            Fragment::Shard { ref_, .. } => ref_,
            Fragment::Fragment { ref_, .. } => ref_,
        }
    }

    /// Get the witness record of a fragment.
    pub fn self_witnessed(&self) -> &Witnessed {
        match self {
            Fragment::Shard { witnessed, .. } => witnessed,
            Fragment::Fragment { witnessed, .. } => witnessed,
        }
    }

    /// Get the data from a fragment.
    pub fn data(&self) -> &str {
        match self {
            Fragment::Shard { data, .. } => data,
            Fragment::Fragment { data, .. } => data,
        }
    }

    /// Get child fragments. Shards have none.
    pub fn children(&self) -> &[Fragment] {
        match self {
            Fragment::Shard { .. } => &[],
            Fragment::Fragment { fragments, .. } => fragments,
        }
    }

    /// Check if a fragment is a shard.
    pub fn is_shard(&self) -> bool {
        matches!(self, Fragment::Shard { .. })
    }

    /// Check if a fragment is a fragment (non-terminal).
    pub fn is_fragment(&self) -> bool {
        matches!(self, Fragment::Fragment { .. })
    }
}

/// Deterministic canonical serialization of a fragment.
pub fn serialize(frag: &Fragment) -> String {
    match frag {
        Fragment::Shard {
            ref_,
            witnessed,
            data,
        } => {
            format!(
                "shard\n{}\n{}\ndata:{}",
                ref_::serialize_ref(ref_),
                witnessed::serialize_witnessed(witnessed),
                data,
            )
        }
        Fragment::Fragment {
            ref_,
            witnessed,
            data,
            fragments,
        } => {
            let children_str: String = fragments
                .iter()
                .map(serialize)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "fragment\n{}\n{}\ndata:{}\nfragments:[{}]",
                ref_::serialize_ref(ref_),
                witnessed::serialize_witnessed(witnessed),
                data,
                children_str,
            )
        }
    }
}

/// Content-address a fragment: SHA-256 of its canonical serialization.
pub fn hash_fragment(frag: &Fragment) -> String {
    let serialized = serialize(frag);
    sha::hash(&serialized).0
}

/// Compute a git-compatible content OID for a fragment.
/// Shard → blob OID, Fragment → tree OID.
/// Witness metadata is NOT included — same content = same OID.
pub fn content_oid(frag: &Fragment) -> String {
    match frag {
        Fragment::Shard { data, .. } => blob_oid(data),
        Fragment::Fragment {
            data, fragments, ..
        } => tree_oid(data, fragments),
    }
}

/// Compute the git blob OID for raw data.
/// SHA-1("blob {len}\0{data}") — matches `git hash-object --stdin`.
pub fn blob_oid(data: &str) -> String {
    use sha1::{Digest, Sha1};
    let header = format!("blob {}\0", data.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute the git tree OID for a fragment with data and children.
/// Builds the same binary tree object that git would, then SHA-1 hashes it.
pub fn tree_oid(data: &str, children: &[Fragment]) -> String {
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
fn build_tree_bytes(data: &str, children: &[Fragment]) -> Vec<u8> {
    let mut entries: Vec<(String, u32, [u8; 20])> = Vec::new();

    // .data entry — the fragment's own data as a blob
    let data_oid_hex = blob_oid(data);
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
