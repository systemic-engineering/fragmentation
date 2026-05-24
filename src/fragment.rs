use crate::encoding::Encode;
use crate::ref_::Ref;
use crate::sha::{HashAlg, Sha};

/// Raw bytes. The default data type for fragments.
pub type Blob = Vec<u8>;

// ---------------------------------------------------------------------------
// Cut 2 (mirror-store.md §4.5 / mirror-native-vcs.md §3.1):
//
// `Fragmentable` splits into two traits:
//
//   `ContentAddressed`  — the OID-computation contract: self_ref + data.
//                         Any type whose identity is its content.
//
//   `TreeShaped`        — the tree-walking extension: children, targets,
//                         shape predicates. Recursive structure.
//
// `Fragmentable` remains as a deprecated alias for `ContentAddressed +
// TreeShaped` so the existing call sites keep working through the
// transition. T2/T3 callers should prefer the narrow trait that matches
// their use; this trait will be removed once the migration completes.
// ---------------------------------------------------------------------------

/// The OID-computation contract. Any type whose identity is its content.
///
/// Minimum trait for content-addressed storage. A backend that just stores
/// and reads typed bytes only needs this.
pub trait ContentAddressed {
    type Data: Encode;
    type Hash: HashAlg;
    fn self_ref(&self) -> &Ref<Self::Hash>;
    fn data(&self) -> &Self::Data;
}

/// The tree-walking contract. Recursive structure with shape predicates.
///
/// Adds the child-listing and lens-target methods on top of
/// [`ContentAddressed`]. Required by `walk`, `merge`, `content_oid`, and
/// anything else that has to traverse a fragment.
pub trait TreeShaped: ContentAddressed
where
    Self: Sized,
{
    fn children(&self) -> &[Self];
    fn is_shard(&self) -> bool {
        self.children().is_empty()
    }
    fn is_fractal(&self) -> bool {
        !self.children().is_empty()
    }
    fn is_lens(&self) -> bool {
        false
    }
    fn targets(&self) -> &[Self::Hash] {
        &[]
    }
}

/// Deprecated alias trait for `ContentAddressed + TreeShaped`. Kept so existing
/// generic bounds (`T: Fragmentable`) continue to compile through the
/// T1→T3 transition. Method-call sites must additionally import the
/// supertrait that carries the method:
///
/// ```text
/// use fragmentation::fragment::{ContentAddressed, TreeShaped};
/// // then call: node.self_ref()  (from ContentAddressed)
/// //           node.children()   (from TreeShaped)
/// ```
///
/// Future code should drop `Fragmentable` entirely in favor of the
/// narrower trait that matches the use. This blanket trait will be removed
/// in 0.2 per docs/specs/mirror-native-vcs.md §3.1.
#[deprecated(
    since = "0.1.1",
    note = "Use `ContentAddressed` (for OID computation) or `TreeShaped` (for tree walking) instead. \
            This blanket trait will be removed in 0.2 per docs/specs/mirror-native-vcs.md §3.1."
)]
pub trait Fragmentable: ContentAddressed + TreeShaped {}

#[allow(deprecated)]
impl<T: ContentAddressed + TreeShaped> Fragmentable for T {}

/// A node in the possibility space.
///
/// Cut 3 (mirror-store.md §4.5): the recursive variant is `Fractal::Branch`,
/// not `Fractal::Fractal`. Removing the doubly-named variant lets grep,
/// rustdoc, and match arms read at the type level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fractal<E = Blob, H: HashAlg = Sha> {
    /// Terminal: self-addressed, carries data, stops.
    Shard { ref_: Ref<H>, data: E },
    /// Self-similar: self-addressed, carries data, contains fractal children.
    Branch {
        ref_: Ref<H>,
        data: E,
        fractal: Vec<Fractal<E, H>>,
    },
    /// Lens: carries data, references external trees by OID. Edges, not containment.
    Lens {
        ref_: Ref<H>,
        data: E,
        target: Vec<H>,
    },
}

impl<H: HashAlg> Fractal<String, H> {
    /// Create a shard from string-like data. Terminal fragment.
    pub fn shard(ref_: Ref<H>, data: impl Into<String>) -> Self {
        Fractal::Shard {
            ref_,
            data: data.into(),
        }
    }

    /// Create a branch from string-like data. Self-similar, contains other fragments.
    pub fn new(ref_: Ref<H>, data: impl Into<String>, fractal: Vec<Fractal<String, H>>) -> Self {
        Fractal::Branch {
            ref_,
            data: data.into(),
            fractal,
        }
    }

    /// Create a lens from string-like data. References external trees by OID.
    pub fn lens(ref_: Ref<H>, data: impl Into<String>, target: Vec<H>) -> Self {
        Fractal::Lens {
            ref_,
            data: data.into(),
            target,
        }
    }
}

impl<E, H: HashAlg> Fractal<E, H> {
    /// Create a shard with typed data. Terminal fragment.
    pub fn shard_typed(ref_: Ref<H>, data: E) -> Self {
        Fractal::Shard { ref_, data }
    }

    /// Create a branch with typed data. Self-similar, contains other fragments.
    pub fn new_typed(ref_: Ref<H>, data: E, fractal: Vec<Fractal<E, H>>) -> Self {
        Fractal::Branch {
            ref_,
            data,
            fractal,
        }
    }

    /// Create a lens with typed data. References external trees by OID.
    pub fn lens_typed(ref_: Ref<H>, data: E, target: Vec<H>) -> Self {
        Fractal::Lens { ref_, data, target }
    }
}

impl<E: Encode, H: HashAlg> ContentAddressed for Fractal<E, H> {
    type Data = E;
    type Hash = H;

    fn self_ref(&self) -> &Ref<H> {
        match self {
            Fractal::Shard { ref_, .. } => ref_,
            Fractal::Branch { ref_, .. } => ref_,
            Fractal::Lens { ref_, .. } => ref_,
        }
    }

    fn data(&self) -> &E {
        match self {
            Fractal::Shard { data, .. } => data,
            Fractal::Branch { data, .. } => data,
            Fractal::Lens { data, .. } => data,
        }
    }
}

impl<E: Encode, H: HashAlg> TreeShaped for Fractal<E, H> {
    fn children(&self) -> &[Fractal<E, H>] {
        match self {
            Fractal::Shard { .. } => &[],
            Fractal::Branch { fractal, .. } => fractal,
            Fractal::Lens { .. } => &[],
        }
    }

    fn is_shard(&self) -> bool {
        matches!(self, Fractal::Shard { .. })
    }

    fn is_fractal(&self) -> bool {
        matches!(self, Fractal::Branch { .. })
    }

    fn is_lens(&self) -> bool {
        matches!(self, Fractal::Lens { .. })
    }

    fn targets(&self) -> &[H] {
        match self {
            Fractal::Lens { target, .. } => target,
            _ => &[],
        }
    }
}

// ---------------------------------------------------------------------------
// Merge — tree merge with caller-provided conflict resolution
// ---------------------------------------------------------------------------

/// Merge two fragment trees node-by-node.
///
/// Same hash → unchanged, skip. Different hash → call `resolve(old, new)`.
/// Children merged positionally: matched children recurse, unmatched
/// children from either side are preserved (information may not be destroyed).
///
/// The resolve function decides conflict winners. The tree walk is free.
/// Tournament rules, holonomy minimization, annealing — all are just
/// different resolve functions.
pub fn merge<F, R>(old: &F, new: &F, resolve: &R) -> F
where
    F: Fragmentable + Reconstructable + Clone,
    F::Data: Clone + crate::encoding::Decode,
    R: Fn(&F, &F) -> F,
{
    // Same content hash → identical, keep old
    if content_oid(old) == content_oid(new) {
        return old.clone();
    }

    // Different content. Resolve this node's data.
    let resolved = resolve(old, new);

    // Merge children positionally
    let old_children = old.children();
    let new_children = new.children();
    let max_len = old_children.len().max(new_children.len());
    let mut merged_children = Vec::with_capacity(max_len);

    for i in 0..max_len {
        match (old_children.get(i), new_children.get(i)) {
            (Some(o), Some(n)) => {
                // Both exist — recurse
                merged_children.push(merge(o, n, resolve));
            }
            (Some(o), None) => {
                // Only in old — preserve (dark dimension)
                merged_children.push(o.clone());
            }
            (None, Some(n)) => {
                // Only in new — preserve (new structure)
                merged_children.push(n.clone());
            }
            (None, None) => unreachable!(),
        }
    }

    // Reconstruct with resolved data and merged children
    F::reconstruct(
        resolved.self_ref().clone(),
        resolved.data().clone(),
        merged_children,
    )
}

// ---------------------------------------------------------------------------
// Reconstruction
// ---------------------------------------------------------------------------

/// Reconstruction from stored parts. Required for read-back from
/// persistent stores (git, disk). Extends Fragmentable with the
/// inverse operation: given ref + data + children → Self.
pub trait Reconstructable: Fragmentable
where
    Self: Sized,
    Self::Data: crate::encoding::Decode,
{
    /// Build a node from its parts. Shard: children is empty.
    /// Fractal: children are the child nodes (already reconstructed).
    fn reconstruct(ref_: Ref<Self::Hash>, data: Self::Data, children: Vec<Self>) -> Self;
}

impl<E: Encode + crate::encoding::Decode, H: HashAlg> Reconstructable for Fractal<E, H> {
    fn reconstruct(ref_: Ref<H>, data: E, children: Vec<Self>) -> Self {
        if children.is_empty() {
            Fractal::Shard { ref_, data }
        } else {
            Fractal::Branch {
                ref_,
                data,
                fractal: children,
            }
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
pub fn lens_oid_bytes<H: HashAlg>(data: &[u8], targets: &[H]) -> String {
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
fn build_lens_tree_bytes<H: HashAlg>(data: &[u8], targets: &[H]) -> Vec<u8> {
    let mut entries: Vec<(String, u32, [u8; 20])> = Vec::new();

    // .data entry
    let data_oid_hex = blob_oid_bytes(data);
    let data_oid_bytes = hex_to_bytes20(&data_oid_hex);
    entries.push((".data".to_string(), 0o100644, data_oid_bytes));

    // .lens entry — newline-separated hex target OIDs
    let lens_content: String = targets
        .iter()
        .map(|h| h.as_str())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha::{HashAlg, Sha};

    /// A mock hash type to prove the generics work.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct MockHash(String);

    impl HashAlg for MockHash {
        fn hash(data: &[u8]) -> Self {
            // Simple mock: just hex-encode the first 8 bytes
            let truncated = &data[..data.len().min(8)];
            MockHash(hex::encode(truncated))
        }

        fn from_hex(hex: impl Into<String>) -> Self {
            MockHash(hex.into())
        }

        fn as_str(&self) -> &str {
            &self.0
        }
    }

    #[test]
    fn fractal_with_custom_hash() {
        // A Fractal using a non-default hash type should compile and work.
        let r = Ref::<MockHash>::new(MockHash("abc".into()), "test");
        let shard: Fractal<String, MockHash> = Fractal::Shard {
            ref_: r,
            data: "hello".into(),
        };
        assert!(shard.is_shard());
    }

    #[test]
    fn fractal_default_hash_is_sha() {
        // Fractal<String> should still default to Sha as hash type.
        let r = Ref::new(Sha("abc".into()), "test");
        let shard: Fractal<String> = Fractal::Shard {
            ref_: r,
            data: "hello".into(),
        };
        assert!(shard.is_shard());
    }

    #[test]
    fn lens_with_custom_hash_targets() {
        let r = Ref::<MockHash>::new(MockHash("abc".into()), "test");
        let targets = vec![MockHash("t1".into()), MockHash("t2".into())];
        let lens: Fractal<String, MockHash> = Fractal::Lens {
            ref_: r,
            data: "link".into(),
            target: targets,
        };
        assert!(lens.is_lens());
        assert_eq!(lens.targets().len(), 2);
        assert_eq!(lens.targets()[0].as_str(), "t1");
    }

    #[test]
    fn fragmentable_targets_returns_hash_type() {
        let r = Ref::<MockHash>::new(MockHash("abc".into()), "test");
        let targets = vec![MockHash("t1".into())];
        let lens: Fractal<String, MockHash> = Fractal::Lens {
            ref_: r,
            data: "link".into(),
            target: targets,
        };
        // The targets() method should return &[MockHash], not &[Sha]
        let t: &[MockHash] = lens.targets();
        assert_eq!(t.len(), 1);
    }

    // -- merge tests --

    fn sha_ref(name: &str) -> Ref<Sha> {
        Ref::new(Sha::hash(name.as_bytes()), name)
    }

    #[test]
    fn merge_identical_trees_returns_old() {
        let a: Fractal<String> = Fractal::shard(sha_ref("x"), "hello");
        let b: Fractal<String> = Fractal::shard(sha_ref("x"), "hello");
        let merged = merge(&a, &b, &|old, _new| old.clone());
        assert_eq!(content_oid(&merged), content_oid(&a));
    }

    #[test]
    fn merge_different_shards_uses_resolve() {
        let a: Fractal<String> = Fractal::shard(sha_ref("a"), "old");
        let b: Fractal<String> = Fractal::shard(sha_ref("b"), "new");
        // resolve: always pick new
        let merged = merge(&a, &b, &|_old, new| new.clone());
        assert_eq!(merged.data(), "new");
    }

    #[test]
    fn merge_different_shards_can_pick_old() {
        let a: Fractal<String> = Fractal::shard(sha_ref("a"), "old");
        let b: Fractal<String> = Fractal::shard(sha_ref("b"), "new");
        // resolve: always pick old
        let merged = merge(&a, &b, &|old, _new| old.clone());
        assert_eq!(merged.data(), "old");
    }

    #[test]
    fn merge_preserves_children_from_old_when_new_has_fewer() {
        let child1: Fractal<String> = Fractal::shard(sha_ref("c1"), "child1");
        let child2: Fractal<String> = Fractal::shard(sha_ref("c2"), "child2");
        let a = Fractal::new(sha_ref("a"), "parent", vec![child1, child2]);
        // new has only one child
        let new_child: Fractal<String> = Fractal::shard(sha_ref("c1-new"), "child1-new");
        let b = Fractal::new(sha_ref("b"), "parent", vec![new_child]);

        let merged = merge(&a, &b, &|_old, new| new.clone());
        // child2 from old must be preserved (dark dimension)
        assert_eq!(merged.children().len(), 2);
        assert_eq!(merged.children()[1].data(), "child2");
    }

    #[test]
    fn merge_preserves_children_from_new_when_old_has_fewer() {
        let child1: Fractal<String> = Fractal::shard(sha_ref("c1"), "child1");
        let a = Fractal::new(sha_ref("a"), "parent", vec![child1]);
        let new_child1: Fractal<String> = Fractal::shard(sha_ref("c1"), "child1");
        let new_child2: Fractal<String> = Fractal::shard(sha_ref("c2-new"), "child2-new");
        let b = Fractal::new(sha_ref("b"), "parent", vec![new_child1, new_child2]);

        let merged = merge(&a, &b, &|_old, new| new.clone());
        assert_eq!(merged.children().len(), 2);
    }

    #[test]
    fn merge_recurses_into_children() {
        let leaf_a: Fractal<String> = Fractal::shard(sha_ref("la"), "leaf-old");
        let leaf_b: Fractal<String> = Fractal::shard(sha_ref("lb"), "leaf-new");
        let a = Fractal::new(sha_ref("a"), "root", vec![leaf_a]);
        let b = Fractal::new(sha_ref("b"), "root", vec![leaf_b]);

        // resolve: always pick new
        let merged = merge(&a, &b, &|_old, new| new.clone());
        assert_eq!(merged.children()[0].data(), "leaf-new");
    }

    #[test]
    fn merge_with_holonomy_strategy() {
        // Simulate holonomy: shorter data = lower loss
        let a: Fractal<String> = Fractal::shard(sha_ref("a"), "long-content-high-loss");
        let b: Fractal<String> = Fractal::shard(sha_ref("b"), "short");

        let merged = merge(&a, &b, &|old, new| {
            // "holonomy" = data length (lower is better)
            if new.data().len() < old.data().len() {
                new.clone()
            } else {
                old.clone()
            }
        });
        assert_eq!(merged.data(), "short");
    }
}
