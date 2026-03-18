use crate::encoding::Encode;
use crate::ref_::Ref;
use crate::sha::Sha;

/// Raw bytes. The default data type for fragments.
pub type Blob = Vec<u8>;

/// The interface for anything content-addressed and self-similar.
/// Turtles all the way down: your children are yourself.
pub trait Fragmentable {
    type Data: Encode;
    fn self_ref(&self) -> &Ref;
    fn data(&self) -> &Self::Data;
    fn children(&self) -> &[Self]
    where
        Self: Sized;
    fn is_shard(&self) -> bool
    where
        Self: Sized,
    {
        self.children().is_empty()
    }
    fn is_fractal(&self) -> bool
    where
        Self: Sized,
    {
        !self.children().is_empty()
    }
    fn is_lens(&self) -> bool
    where
        Self: Sized,
    {
        false
    }
    fn targets(&self) -> &[Sha] {
        &[]
    }
}

/// A node in the possibility space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fractal<E = Blob> {
    /// Terminal: self-addressed, carries data, stops.
    Shard { ref_: Ref, data: E },
    /// Self-similar: self-addressed, carries data, contains fractal children.
    Fractal {
        ref_: Ref,
        data: E,
        fractal: Vec<Fractal<E>>,
    },
    /// Lens: carries data, references external trees by OID. Edges, not containment.
    Lens {
        ref_: Ref,
        data: E,
        target: Vec<Sha>,
    },
}

impl Fractal<String> {
    /// Create a shard from string-like data. Terminal fragment.
    pub fn shard(ref_: Ref, data: impl Into<String>) -> Self {
        Fractal::Shard {
            ref_,
            data: data.into(),
        }
    }

    /// Create a fractal from string-like data. Self-similar, contains other fragments.
    pub fn new(ref_: Ref, data: impl Into<String>, fractal: Vec<Fractal<String>>) -> Self {
        Fractal::Fractal {
            ref_,
            data: data.into(),
            fractal,
        }
    }

    /// Create a lens from string-like data. References external trees by OID.
    pub fn lens(ref_: Ref, data: impl Into<String>, target: Vec<Sha>) -> Self {
        Fractal::Lens {
            ref_,
            data: data.into(),
            target,
        }
    }
}

impl<E> Fractal<E> {
    /// Create a shard with typed data. Terminal fragment.
    pub fn shard_typed(ref_: Ref, data: E) -> Self {
        Fractal::Shard { ref_, data }
    }

    /// Create a fractal with typed data. Self-similar, contains other fragments.
    pub fn new_typed(ref_: Ref, data: E, fractal: Vec<Fractal<E>>) -> Self {
        Fractal::Fractal {
            ref_,
            data,
            fractal,
        }
    }

    /// Create a lens with typed data. References external trees by OID.
    pub fn lens_typed(ref_: Ref, data: E, target: Vec<Sha>) -> Self {
        Fractal::Lens { ref_, data, target }
    }
}

impl<E: Encode> Fragmentable for Fractal<E> {
    type Data = E;

    fn self_ref(&self) -> &Ref {
        match self {
            Fractal::Shard { ref_, .. } => ref_,
            Fractal::Fractal { ref_, .. } => ref_,
            Fractal::Lens { ref_, .. } => ref_,
        }
    }

    fn data(&self) -> &E {
        match self {
            Fractal::Shard { data, .. } => data,
            Fractal::Fractal { data, .. } => data,
            Fractal::Lens { data, .. } => data,
        }
    }

    fn children(&self) -> &[Fractal<E>] {
        match self {
            Fractal::Shard { .. } => &[],
            Fractal::Fractal { fractal, .. } => fractal,
            Fractal::Lens { .. } => &[],
        }
    }

    fn is_shard(&self) -> bool {
        matches!(self, Fractal::Shard { .. })
    }

    fn is_fractal(&self) -> bool {
        matches!(self, Fractal::Fractal { .. })
    }

    fn is_lens(&self) -> bool {
        matches!(self, Fractal::Lens { .. })
    }

    fn targets(&self) -> &[Sha] {
        match self {
            Fractal::Lens { target, .. } => target,
            _ => &[],
        }
    }
}

/// Compute a git-compatible content OID for any Fragmentable.
/// Shard -> blob OID, Fractal -> tree OID, Lens -> tree OID (.data + .lens).
/// Witness metadata is NOT included -- same content = same OID.
pub fn content_oid<F: Fragmentable>(frag: &F) -> String {
    if frag.is_shard() {
        blob_oid_bytes(&frag.data().encode())
    } else if frag.is_lens() {
        lens_oid_bytes(&frag.data().encode(), frag.targets())
    } else {
        tree_oid_bytes(&frag.data().encode(), frag.children())
    }
}

/// Compute the git tree OID for a Lens with data and target OIDs.
/// Builds a git tree with `.data` blob + `.lens` blob (newline-separated hex OIDs).
pub fn lens_oid_bytes(data: &[u8], targets: &[Sha]) -> String {
    use sha1::{Digest, Sha1};

    let tree_bytes = build_lens_tree_bytes(data, targets);
    let header = format!("tree {}\0", tree_bytes.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(&tree_bytes);
    hex::encode(hasher.finalize())
}

/// Build the raw bytes of a git tree object for a Lens (without header).
/// Entries: ".data" blob + ".lens" blob (newline-separated hex target OIDs).
fn build_lens_tree_bytes(data: &[u8], targets: &[Sha]) -> Vec<u8> {
    let mut entries: Vec<(String, u32, [u8; 20])> = Vec::new();

    // .data entry
    let data_oid_hex = blob_oid_bytes(data);
    let data_oid_bytes = hex_to_bytes20(&data_oid_hex);
    entries.push((".data".to_string(), 0o100644, data_oid_bytes));

    // .lens entry — newline-separated hex target OIDs
    let lens_content: String = targets
        .iter()
        .map(|sha| sha.0.as_str())
        .collect::<Vec<&str>>()
        .join("\n");
    let lens_oid_hex = blob_oid_bytes(lens_content.as_bytes());
    let lens_oid_raw = hex_to_bytes20(&lens_oid_hex);
    entries.push((".lens".to_string(), 0o100644, lens_oid_raw));

    // Git sorts tree entries by name (byte order)
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut buf = Vec::new();
    for (name, mode, oid) in &entries {
        buf.extend_from_slice(format!("{} {}\0", mode_to_string(*mode), name).as_bytes());
        buf.extend_from_slice(oid);
    }
    buf
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
pub fn tree_oid<F: Fragmentable>(data: &str, children: &[F]) -> String {
    tree_oid_bytes(data.as_bytes(), children)
}

/// Compute the git tree OID for a fragment with byte data and children.
pub fn tree_oid_bytes<F: Fragmentable>(data: &[u8], children: &[F]) -> String {
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
fn build_tree_bytes<F: Fragmentable>(data: &[u8], children: &[F]) -> Vec<u8> {
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
