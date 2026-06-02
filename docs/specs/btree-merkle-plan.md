# B-tree normalized Merkle tree for Fractal<MirrorData>

**Author:** Mara
**Date:** 2026-04-15
**Status:** Plan
**Crate:** `fragmentation`
**Consumers:** `mirror` (MirrorFragment = Fractal<MirrorData>), `coincidence` (EigenStore)

---

## Problem

`Fractal<D>` is already a Merkle tree structurally — each node has data and
children, and `content_oid()` hashes data + children recursively. But it is
**not B-tree normalized**:

1. **Unbounded branching.** A node can have 0..N children. A grammar file with
   200 declarations produces a root with 200 direct children. No intermediate
   structure.

2. **Insertion-order-dependent shape.** Two trees with the same set of children
   but different insertion order produce different `Vec<Fractal>` orderings,
   different positional names ("0000", "0001", ...), and therefore different
   tree OIDs. Same content, different address.

3. **O(n) diff.** The current `diff.rs` compares children positionally. Change
   one child in a list of 200 and the diff walks all 200. No hash-based
   short-circuit at intermediate levels.

4. **Flat disk layout.** `FrgmntStore` writes each node as a single file under
   `objects/<2-char-fanout>/<rest>`. No structural relationship between files.
   Reading a tree requires N random reads with no locality.

## Goal

A B-tree normalized Merkle representation where:

- Same set of children (by OID) always produces the same tree shape
- Branching factor is bounded (default: 32)
- Diff is O(log n) for single-node changes
- Disk layout preserves locality within subtrees
- Uses `Oid::hash()` (CoincidenceHash<3>) from prism, not SHA-1/SHA-256
- Round-trips through git objects (tree/blob) for interop

---

## Current state audit

### content_oid (fragment.rs)

```
content_oid(frag) =
  shard  -> SHA-1("blob {len}\0{data.encode()}")
  fractal -> SHA-1("tree {len}\0{.data blob + 0000..NNNN children}")
  lens   -> SHA-1("tree {len}\0{.data blob + .lens blob}")
```

Uses SHA-1 with git-format headers. Children named positionally: `0000`,
`0001`, etc. Sorted by name (which is insertion order for sequential indices).

### FrgmntStore (frgmnt_store.rs)

```
.frgmnt/
  objects/<2-char>/<rest>   — flat file per node, data bytes only
  refs/<name>               — named pointers (plain text OID)
```

- `write_to_disk()` writes `data.encode()` bytes only — no children.
- `read_from_disk()` reconstructs with `children: vec![]` — tree structure lost.
- Persistent mode is shard-only. Trees cannot round-trip through FrgmntStore.

### GitStore (git.rs)

```
write_tree() -> git tree object:
  .data blob (data.encode())
  0000 child_oid (tree or blob)
  0001 child_oid
  ...
```

- Full tree round-trip through git ODB via `write_tree`/`read_tree`.
- Children are positional — insertion order determines tree shape.

### diff (diff.rs)

```
diff(old, new):
  content_oid(old) == content_oid(new) -> Unchanged  (O(1) shortcircuit)
  otherwise -> Modified + diff_children positionally  (O(n))
```

- Root shortcircuit works. But children are compared by index, not by OID.
- No intermediate hash nodes to skip unchanged subtrees.

### Coincidence hash (prism core/src/oid.rs)

```
Oid::hash(bytes) -> CoincidenceHash<3> -> SHA-256 compressed -> 64 hex chars
```

- Three independent projection observers in 16-dimensional space.
- Deterministic. Cross-version stable (pinned test value for b"prism").
- This is the hash function for the Merkle tree.

---

## Design

### 1. MerkleNode<D, H>

A new type that wraps `Fractal<D, H>` into B-tree normalized form.

```rust
// src/merkle.rs

/// A B-tree normalized Merkle node.
///
/// Invariants:
/// - children sorted by OID (lexicographic on hex string)
/// - children.len() <= BRANCHING_FACTOR
/// - intermediate nodes have kind = Intermediate (no user data)
/// - leaf/interior OID = hash(data || sorted child OIDs)
pub struct MerkleNode<D: Encode, H: HashAlg = Sha> {
    oid: H,
    kind: MerkleKind<D>,
    children: Vec<MerkleNode<D, H>>,
}

enum MerkleKind<D> {
    /// User data node (from a Fractal)
    Data(D),
    /// Intermediate B-tree node (no user data, only structure)
    Intermediate,
}
```

### 2. Branching factor

```rust
/// Default branching factor. 32 = 5 bits per level.
/// A tree with 1M leaves has depth ceil(log32(1_000_000)) = 4.
pub const DEFAULT_BRANCHING_FACTOR: usize = 32;
```

Configurable at construction time, but 32 is the default for all stores.

### 3. OID computation

```rust
/// Leaf OID:
///   hash(data.encode())
///
/// Interior OID:
///   hash(data.encode() || child_0.oid || child_1.oid || ... || child_n.oid)
///
/// Intermediate OID:
///   hash(0x00 || child_0.oid || child_1.oid || ... || child_n.oid)
///   (0x00 domain-separates intermediate from data nodes)
fn compute_oid<D: Encode, H: HashAlg>(kind: &MerkleKind<D>, children: &[H]) -> H {
    let mut buf = Vec::new();
    match kind {
        MerkleKind::Data(d) => buf.extend_from_slice(&d.encode()),
        MerkleKind::Intermediate => buf.push(0x00),
    }
    for child_oid in children {
        buf.extend_from_slice(child_oid.as_str().as_bytes());
    }
    H::hash(&buf)
}
```

This uses `HashAlg::hash()` which is `Sha::hash()` (SHA-256) by default. When
the mirror crate uses this with prism's Oid, it will implement `HashAlg` for
`Oid` and the coincidence hash flows through automatically.

### 4. Normalization: Fractal -> MerkleNode

```rust
/// Convert a Fractal<D> to a B-tree normalized MerkleNode<D>.
///
/// 1. Recursively normalize all children
/// 2. Sort children by OID
/// 3. If children.len() > branching_factor, split into intermediate nodes
/// 4. Compute the node's OID from data + sorted child OIDs
pub fn normalize<D: Encode + Clone, H: HashAlg>(
    fractal: &Fractal<D, H>,
    branching_factor: usize,
) -> MerkleNode<D, H>
```

**Split algorithm** when children exceed branching factor:

```
Given sorted children [c0, c1, ..., c_{n-1}] where n > B:

1. Chunk into groups of B: [c0..cB], [cB..c2B], ..., [c_{n-B}..c_n]
2. Each chunk becomes an Intermediate node
3. If the number of intermediates > B, recurse (split the intermediates)
4. The result is a balanced B-tree of intermediates with data leaves
```

This is the same algorithm as git's tree packing and IPFS's HAMT.

### 5. Denormalization: MerkleNode -> Fractal

```rust
/// Convert a MerkleNode<D> back to a Fractal<D>.
///
/// Intermediate nodes are transparent — their children are
/// flattened into the parent's child list. The Fractal sees
/// only data nodes.
pub fn denormalize<D: Encode + Clone, H: HashAlg>(
    node: &MerkleNode<D, H>,
) -> Fractal<D, H>
```

Intermediate nodes are an internal optimization. The user-visible Fractal
never contains them.

### 6. Disk layout

Two strategies, both compatible:

**Strategy A: FrgmntStore extension (no git dependency)**

```
.frgmnt/
  objects/<2-char>/<rest>     — node data, one file per node
  trees/<2-char>/<rest>       — child list, one file per interior node
  refs/<name>                 — named pointers
```

Each `trees/` file contains the sorted child OIDs, newline-separated. This
is the B-tree structure on disk. Reading a tree: read the root's tree file,
then recursively read children.

Extension to `FrgmntStore`:

```rust
impl<N: Reconstructable + Clone> FrgmntStore<N> {
    /// Write a MerkleNode tree to disk.
    pub fn write_merkle(&self, node: &MerkleNode<N::Data, N::Hash>) -> Result<(), Error>;

    /// Read a MerkleNode tree from disk by root OID.
    pub fn read_merkle(&self, oid: &str) -> Result<MerkleNode<N::Data, N::Hash>, Error>;
}
```

**Strategy B: Git interop (git feature)**

MerkleNode maps directly to git tree objects:

- Data node -> git tree with `.data` blob + numbered children (existing format)
- Intermediate node -> git tree with `.intermediate` marker blob + numbered children
- The `.intermediate` marker is an empty blob — its presence signals "skip this
  level when denormalizing"

This preserves backward compatibility: existing trees without `.intermediate`
markers are read as flat (non-normalized) trees.

### 7. Diff algorithm

```rust
/// O(log n) diff for B-tree normalized Merkle trees.
///
/// 1. Compare root OIDs — if equal, return Unchanged (O(1))
/// 2. Compare child OID lists — only recurse into children whose OIDs differ
/// 3. For intermediate nodes, recurse transparently
/// 4. Collect changes as Added/Removed/Modified/Unchanged
pub fn merkle_diff<D: Encode + Clone, H: HashAlg>(
    old: &MerkleNode<D, H>,
    new: &MerkleNode<D, H>,
) -> Vec<Change<MerkleNode<D, H>>>
```

The key insight: because children are sorted by OID, the diff can use a
merge-join (two sorted lists) instead of positional comparison. This makes
it O(k log n) where k is the number of changed nodes, not O(n).

### 8. HashAlg bridge for Oid

The mirror crate needs `Oid` to implement `HashAlg` so that
`MerkleNode<MirrorData, Oid>` uses coincidence hashing:

```rust
// In mirror or coincidence crate:
impl fragmentation::sha::HashAlg for prism::Oid {
    fn hash(data: &[u8]) -> Self { Oid::hash(data) }
    fn from_hex(hex: impl Into<String>) -> Self { Oid::new(hex) }
    fn as_str(&self) -> &str { self.as_str() }
}
```

This is a one-time bridge. Once it exists, `Fractal<MirrorData, Oid>` and
`MerkleNode<MirrorData, Oid>` both use CoincidenceHash<3>.

---

## Implementation plan

### Arc 1: MerkleNode type + OID computation

**Files:** `src/merkle.rs`, `src/lib.rs`

| # | Task | Test |
|---|------|------|
| 1.1 | Add `src/merkle.rs` with `MerkleNode`, `MerkleKind`, `DEFAULT_BRANCHING_FACTOR` | `merkle_node_data_variant`, `merkle_node_intermediate_variant` |
| 1.2 | Implement `compute_oid()` | `leaf_oid_is_hash_of_data`, `interior_oid_includes_children`, `intermediate_oid_domain_separated` |
| 1.3 | Implement `MerkleNode::new_leaf()`, `MerkleNode::new_interior()` | `new_leaf_has_no_children`, `new_interior_has_children` |
| 1.4 | Determinism test: same data + same children = same OID regardless of construction | `oid_deterministic` |
| 1.5 | Add `pub mod merkle;` to `lib.rs` | compile gate |

### Arc 2: Normalize + Denormalize

**Files:** `src/merkle.rs`

| # | Task | Test |
|---|------|------|
| 2.1 | `normalize()` for shards (no children) | `normalize_shard_is_leaf` |
| 2.2 | `normalize()` for fractals with <= B children | `normalize_small_fractal_no_split` |
| 2.3 | `normalize()` sorts children by OID | `normalize_sorts_children_by_oid` |
| 2.4 | `normalize()` splits when > B children, creates intermediates | `normalize_large_fractal_splits_into_intermediates` |
| 2.5 | `normalize()` recursive split (> B^2 children) | `normalize_deep_split_two_levels` |
| 2.6 | `denormalize()` for leaf | `denormalize_leaf_is_shard` |
| 2.7 | `denormalize()` for interior (no intermediates) | `denormalize_interior_preserves_children` |
| 2.8 | `denormalize()` flattens intermediates | `denormalize_flattens_intermediates` |
| 2.9 | Round-trip: `denormalize(normalize(f)) == f` (modulo child order) | `roundtrip_normalize_denormalize` |
| 2.10 | Same-content invariant: `normalize(f1).oid == normalize(f2).oid` when f1 and f2 have same children in different order | `same_content_same_oid_regardless_of_order` |

### Arc 3: Merkle diff

**Files:** `src/merkle.rs` (or `src/merkle_diff.rs`)

| # | Task | Test |
|---|------|------|
| 3.1 | `merkle_diff()` for identical trees (root OID match) | `diff_identical_is_unchanged` |
| 3.2 | `merkle_diff()` for single leaf change | `diff_single_leaf_change` |
| 3.3 | `merkle_diff()` for added child | `diff_added_child` |
| 3.4 | `merkle_diff()` for removed child | `diff_removed_child` |
| 3.5 | `merkle_diff()` skips unchanged subtrees (verify recursion count) | `diff_skips_unchanged_subtrees` |
| 3.6 | `merkle_diff()` through intermediate nodes | `diff_through_intermediates` |

### Arc 4: FrgmntStore disk persistence

**Files:** `src/frgmnt_store.rs`

| # | Task | Test |
|---|------|------|
| 4.1 | Add `trees/` directory creation in `FrgmntStore::open()` | `open_creates_trees_dir` |
| 4.2 | `write_merkle()` writes data files + tree files | `write_merkle_creates_files` |
| 4.3 | `read_merkle()` reads back a normalized tree | `read_merkle_roundtrip` |
| 4.4 | `read_merkle()` handles intermediates | `read_merkle_with_intermediates` |
| 4.5 | `flush_merkle()` writes entire cache as merkle trees | `flush_merkle_persists_all` |

### Arc 5: Git interop

**Files:** `src/git.rs`

| # | Task | Test |
|---|------|------|
| 5.1 | `write_merkle_tree()` writes MerkleNode to git ODB | `write_merkle_to_git` |
| 5.2 | `.intermediate` marker blob for intermediate nodes | `intermediate_marker_in_git_tree` |
| 5.3 | `read_merkle_tree()` reads back, detecting `.intermediate` | `read_merkle_from_git` |
| 5.4 | Round-trip: write -> read -> compare | `git_merkle_roundtrip` |
| 5.5 | Backward compat: old trees (no `.intermediate`) read as flat | `legacy_tree_reads_as_flat` |

### Arc 6: HashAlg bridge for Oid

**Files:** `mirror/src/oid_bridge.rs` or `coincidence/src/hash_bridge.rs`

| # | Task | Test |
|---|------|------|
| 6.1 | Implement `HashAlg for Oid` | `oid_implements_hash_alg` |
| 6.2 | `MerkleNode<MirrorData, Oid>` compiles and hashes correctly | `mirror_merkle_uses_coincidence_hash` |
| 6.3 | Normalize a `Fractal<MirrorData, Oid>` | `normalize_mirror_fragment` |

---

## Invariants

These must hold at all times and are tested directly:

1. **Determinism.** `normalize(f).oid` is a pure function of the set of
   descendant OIDs. Insertion order does not affect the result.

2. **Bounded branching.** No node in a normalized tree has more than
   `branching_factor` children.

3. **Sorted children.** Children are sorted by OID at every level.

4. **Transparent intermediates.** `denormalize(normalize(f))` produces a
   Fractal with the same data nodes as `f`, sorted by OID.

5. **Content addressing.** Changing one leaf changes the OID of every
   ancestor up to the root. No other OIDs change.

6. **Diff locality.** `merkle_diff` visits at most O(log n) nodes for a
   single-leaf change in a tree of n leaves.

---

## Non-goals (this plan)

- Pack files (multiple nodes in one file). Future optimization.
- Streaming/incremental normalization. Batch-only for now.
- Concurrent merkle writes. The `BoundedStore` mutex is sufficient.
- Migration of existing `.frgmnt` stores. New stores use the new format;
  old stores continue to work (flat, non-normalized).

---

## Dependencies

- `fragmentation` crate (this repo): no new deps. `sha2`, `hex`, `dashmap`
  already present.
- `prism` crate: `Oid::hash()` already exists. Only needs `HashAlg` impl.
- `mirror` crate: `MirrorData` already implements `Encode`/`Decode`. No changes.

## Risk

The main risk is the OID computation changing, which would invalidate all
existing content addresses. Mitigation: the `compute_oid()` function is new
and only used for normalized trees. Existing `content_oid()` is untouched.
The two hash functions coexist. Migration is explicit, not implicit.
