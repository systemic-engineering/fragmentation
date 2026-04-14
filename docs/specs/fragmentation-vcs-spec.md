# Fragmentation as Git-Compatible Version Control System

**Author:** Mara <mara@systemic.engineer>
**Date:** 2026-04-14
**Status:** Draft

---

## 0. Thesis

A `Fractal<D>` tree and a git tree are the same mathematical object: content-addressed,
sorted, self-similar. The fragmentation crate already computes git-compatible SHA-1 OIDs
(`blob_oid`, `tree_oid`, `content_oid`), writes native git objects via `git2`, and has a
commit model with parent chains and witness metadata. The gap is not structural. It is
that fragmentation does not yet carry its own loss semantics through store operations,
and the optic hierarchy that already exists in the codebase (`prism`, `framework`) is
not wired as a dependency.

This spec closes both gaps: prism becomes a dependency, and fragmentation becomes a
self-sufficient VCS that happens to speak git's wire protocol.

---

## 1. Prism as Dependency

### 1.1 Cargo dependency

```toml
[dependencies]
terni = { path = "../prism/imperfect" }
prism = { path = "../prism/core", optional = true }

[features]
optics = ["dep:prism"]
```

`terni` (the `Imperfect<T, E, L>` type) is unconditional. Every store operation
returns `Imperfect`. The `prism` core (Beam, Prism trait, Optic) is feature-gated
behind `optics` for crates that want the pipeline DSL.

### 1.2 StoreLoss

A new loss type measuring what a store operation cost.

```rust
/// What a store operation lost.
///
/// Three independent dimensions:
/// - dedup_ratio: fraction of bytes that were already present (0.0 = all new, 1.0 = fully deduped)
/// - compression: fraction of bytes saved by encoding (0.0 = no saving, 1.0 = total)
/// - fragmentation: ratio of wasted space to useful space in the object directory
///
/// These are not errors. They are measurements of where information went.
pub struct StoreLoss {
    pub dedup_ratio: f64,
    pub compression: f64,
    pub fragmentation_level: f64,
}
```

`StoreLoss` implements `terni::Loss`:
- `zero()`: all fields 0.0 (no loss).
- `total()`: all fields 1.0 (total loss).
- `combine`: element-wise max (loss only grows).
- `is_zero()`: all fields == 0.0.

### 1.3 Store operations return Imperfect

Current `FrgmntStore` signatures:

```rust
// current
pub fn insert(&self, key: String, value: N, size_bytes: usize);
pub fn get(&self, key: &str) -> Option<N>;
pub fn insert_persistent(&self, key: String, value: N, size_bytes: usize);
pub fn get_persistent(&self, key: &str) -> Option<N>;
```

Proposed signatures:

```rust
// proposed
pub fn insert(&self, key: String, value: N, size_bytes: usize)
    -> Imperfect<Shard<N>, StoreError, StoreLoss>;

pub fn get(&self, key: &str)
    -> Imperfect<Shard<N>, StoreError, StoreLoss>;

pub fn insert_persistent(&self, key: String, value: N, size_bytes: usize)
    -> Imperfect<Shard<N>, StoreError, StoreLoss>;

pub fn get_persistent(&self, key: &str)
    -> Imperfect<Shard<N>, StoreError, StoreLoss>;
```

Where `Shard<N>` wraps a value with its computed OID (matching mirror's `Shard<V>`
pattern in `mirror/src/store.rs`).

Semantics:
- **Success**: value stored/retrieved, zero loss.
- **Partial**: value stored/retrieved, but something cost more than zero:
  - `dedup_ratio > 0`: the value was already present (content-addressed dedup).
  - `compression > 0`: encoding shrank the data.
  - `fragmentation_level > 0`: eviction occurred (bounded store).
- **Failure**: I/O error, decode error, or key not found.

The store IS a fold: each `insert` accumulates loss across the store's lifetime.
The loss tells you the store's health without inspecting its internals.

### 1.4 StoreError

```rust
pub enum StoreError {
    Io(std::io::Error),
    Decode(String),
    NotFound(String),
    Evicted(String),
}
```

Replaces the current `frgmnt_store::Error` and adds `NotFound` / `Evicted` variants
that were previously expressed as `Option::None`.

---

## 2. Fractal Implements Optics (framework/prism feature gate)

When the `optics` feature is enabled, `Fractal<D>` gains optic implementations.
These map the existing optic hierarchy to fragment tree navigation.

### 2.1 The hierarchy

| Optic | Type | Meaning |
|-------|------|---------|
| Lens | `Fractal<D> -> D` | Focus on data. Always present. Total, lawful. |
| Prism (optic) | `Fractal<D> -> D` | Project into data. Fails on wrong variant. Partial. |
| Prism (optic) | `Fractal<D> -> Fractal<D>` | Project into child by OID. Fails if OID not found. |
| Traversal | `Fractal<D> -> Vec<D>` | Walk all data nodes depth-first. |
| AffineTraversal | `Fractal<D> -> Option<D>` | Walk to a specific node by path. May not exist. |

### 2.2 Lens: data focus

```rust
impl<D: Clone> prism::Prism for FractalDataLens<D> {
    type Input = Optic<(), Fractal<D>>;
    type Focused = Optic<Fractal<D>, D>;
    type Projected = Optic<D, D>;
    type Refracted = Optic<D, Fractal<D>>;

    fn focus(&self, beam: Self::Input) -> Self::Focused { /* extract .data() */ }
    fn project(&self, beam: Self::Focused) -> Self::Projected { /* identity on D */ }
    fn refract(&self, beam: Self::Projected) -> Self::Refracted { /* rebuild Fractal with new D */ }
}
```

This is total: every Fractal variant (Shard, Fractal, Lens) carries data. The data
Lens never fails.

### 2.3 Prism: child by OID

```rust
impl<D: Clone> prism::Prism for FractalChildPrism<D> {
    type Input = Optic<(), (Fractal<D>, String)>;  // (tree, target_oid)
    type Focused = Optic<(Fractal<D>, String), Fractal<D>>;
    type Projected = Optic<Fractal<D>, Fractal<D>>;
    type Refracted = Optic<Fractal<D>, Fractal<D>>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        // Walk children, find the one whose content_oid matches target_oid.
        // Failure beam if not found.
    }
}
```

This is partial: the child may not exist. This maps exactly to the optic Prism
(preview may fail, review always succeeds).

### 2.4 Traversal: walk all nodes

```rust
impl<D: Clone> prism::Prism for FractalTraversal<D> {
    type Input = Optic<(), Fractal<D>>;
    type Focused = Optic<Fractal<D>, Vec<D>>;
    type Projected = Optic<Vec<D>, Vec<D>>;
    type Refracted = Optic<Vec<D>, Vec<Fractal<D>>>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        // walk::collect, extract data from each node
    }
}
```

This is the multi-site observation from `walk.rs`. The existing `walk::collect`
and `walk::fold` are already traversals. This wires them into the Beam pipeline.

### 2.5 Singularity maps to optic hierarchy

The existing `Singularity` trait already documents this mapping:

| Singularity | Optic | Loss |
|-------------|-------|------|
| Identity (`Fractal::collapse`) | Iso | Infallible (zero) |
| WitnessedSingularity::collapse | Lens | observer metadata |
| WitnessedSingularity::prism | Prism | Option (may not see through) |
| WitnessedSingularity::traversal | Traversal | partial views accumulated |

The optic implementations formalize what the Singularity trait already proves.

---

## 3. Fragmentation as Git-Compatible VCS

### 3.1 The isomorphism

| Git concept | Fragmentation equivalent | Notes |
|-------------|------------------------|-------|
| blob | `Fractal::Shard` | Terminal node. Content-addressed. |
| tree | `Fractal::Fractal` | Recursive node. Sorted children. `.data` entry. |
| commit | `Commit<N, H>` | Root or Child. Witnessed. Content-addressed SHA. |
| ref | `Repo::update_ref` / `FrgmntStore::set_ref` | Named pointer to a commit hash. |
| tag | (not yet modeled) | A ref with metadata. Straightforward extension. |
| object store | `FrgmntStore` / `GitStore` | Fan-out by first 2 hex chars. |

The isomorphism is already proven in code:
- `fragment::content_oid` produces SHA-1 hashes using git's `blob {len}\0{data}`
  and `tree {len}\0{entries}` format.
- `git::write_tree` writes `Fractal` nodes as native git objects via `git2`.
- `git::read_tree` reconstructs `Fractal` trees from git objects.
- `commit::compute_commit_sha` produces git-compatible commit hashes.
- `FrgmntStore` uses `.frgmnt/objects/` with fan-out matching `.git/objects/`.
- `FrgmntStore::set_ref` / `get_ref` stores named refs matching `.git/refs/`.

### 3.2 What is missing

The current crate has all the pieces but lacks the orchestration layer that
makes it a VCS. Specifically:

1. **Working tree tracking.** Git tracks a working tree and computes diffs
   against it. Fragmentation has `diff.rs` but no working tree concept.

2. **Index / staging area.** Git's index is the staging area between working
   tree and commit. Fragmentation has no equivalent.

3. **Branch management.** `Repo::update_ref` can point refs at commits, but
   there is no HEAD, no branch switching, no detached HEAD.

4. **Merge.** `diff.rs` has positional child comparison but no three-way merge.

5. **Transport.** Git's smart HTTP / SSH protocols for push/pull. The current
   `git` feature uses `git2` for local object read/write but not transport.

### 3.3 Architecture: `.frgmnt/` as the store

```
.frgmnt/
  objects/          # content by OID, fan-out by first 2 hex chars
    ab/
      cdef1234...   # serialized fragment node
  refs/
    heads/
      main          # plain text: commit OID
      feature       # plain text: commit OID
    tags/
      v0.1.0        # plain text: commit OID
  HEAD              # ref: refs/heads/main
  config            # store configuration (hash algorithm, etc.)
```

This is `.git/` reimagined. The object format is the same (git-compatible SHA-1
hashing). The difference is that fragmentation stores trees with `.data` entries
and numbered children, while git stores trees with filename entries. The
`write_tree_named` variant already bridges this for filesystem trees.

### 3.4 Commit model

Already implemented. A `Commit<N, H>` is either `Root` (no parent) or `Child`
(has parent). The `Draft` builder pattern creates commits:

```
Draft::root(message, tree)            -> Commit::Root
commit.child(message, tree)           -> Draft with Parent
draft.commit(repo, committer, time)   -> Commit::Child
```

The `Witnessed` metadata (Author, Committer, Timestamp) maps directly to git
commit metadata. `compute_commit_sha` produces git-identical commit hashes.

### 3.5 Structural diff (O(log n) for B-tree normalized trees)

The current `diff.rs` compares two trees by content OID at each node:

```
if content_oid(old) == content_oid(new) -> Unchanged (skip entire subtree)
else -> Modified, recurse into children
```

This is already O(log n) for balanced trees: when most subtrees are unchanged,
the diff only walks the spine of changed nodes. For B-tree normalized trees
(which fragmentation's encoding produces via document -> paragraph -> sentence
-> word -> char decomposition), the branching factor ensures logarithmic depth.

**Three-way merge** extends this:

```
merge(base, ours, theirs):
  if oid(base) == oid(ours)  -> take theirs  (we didn't change it)
  if oid(base) == oid(theirs) -> take ours   (they didn't change it)
  if oid(ours) == oid(theirs) -> take either  (same change)
  else -> conflict at this node, recurse into children
```

This is the same structural merge that git performs on trees, but operating on
`Fractal` nodes instead of filesystem entries. The content-addressing makes it
cheap: equal OID means equal content, no byte comparison needed.

### 3.6 Named refs as branches

The `Repo` trait already has `update_ref` and `resolve_ref`. Branches are refs
under `refs/heads/`. HEAD is a symbolic ref pointing at the current branch.

```rust
pub struct Head {
    /// Symbolic: "ref: refs/heads/main"
    /// Detached: raw commit OID
    target: HeadTarget,
}

pub enum HeadTarget {
    Branch(String),     // "refs/heads/main"
    Detached(String),   // commit OID
}
```

### 3.7 The VCS operations

| Operation | Implementation | Complexity |
|-----------|---------------|------------|
| `init` | Create `.frgmnt/` directory structure | O(1) |
| `commit` | `Draft::root` or `commit.child`, write tree, update HEAD | O(n) first, O(delta) subsequent |
| `diff` | `diff::diff(old_root, new_root)` | O(log n) for balanced trees |
| `log` | Walk parent chain from HEAD | O(commits) |
| `branch` | `set_ref("refs/heads/name", oid)` | O(1) |
| `checkout` | Update HEAD, reconstruct working tree | O(n) |
| `merge` | Three-way structural merge on Fractal trees | O(log n) for balanced trees |
| `status` | Diff working tree against HEAD's root | O(delta) |

---

## 4. The Bridge: MirrorOid and git SHA

### 4.1 Dual addressing

Mirror uses SHA-512 (`MirrorOid` wrapping `Oid` from prism). Git uses SHA-1
(fragmentation's `Sha`). A single `Fractal` can carry both addresses.

The `HashAlg` trait in `fragmentation::sha` is already generic:

```rust
pub trait HashAlg: Clone + Debug + PartialEq + Eq + Hash {
    fn hash(data: &[u8]) -> Self;
    fn from_hex(hex: impl Into<String>) -> Self;
    fn as_str(&self) -> &str;
}
```

And `Fractal<E, H: HashAlg = Sha>` is already parameterized over the hash type.

### 4.2 ForeignKey bridge

Mirror's `store.rs` already defines the `ForeignKey` trait:

```rust
pub trait ForeignKey {
    fn foreign_hex(&self) -> Option<&str>;
}
```

The bridge works as follows:

```
MirrorOid (SHA-512)  <-- home hash, used for spectral operations
     |
     | ForeignKey
     v
Sha (SHA-1)          <-- visitor hash, used for git interop
```

A `Shard<V>` addressed by `MirrorOid` can carry a `ForeignKey` to the git SHA-1
world. Home produces visitors. Visitors do not produce home. This is directional
by design: the spectral hash is the source of truth, the git hash is a projection.

### 4.3 Dual-OID Fractal

```rust
/// A Fractal with both native and git-compatible OIDs.
pub struct DualFractal<D> {
    /// The fractal tree, addressed by native hash.
    pub fractal: Fractal<D>,
    /// The git-compatible SHA-1 OID, computed lazily.
    pub git_oid: Option<String>,
}
```

The `git_oid` is computed by `fragment::content_oid` (which uses SHA-1 in git
blob/tree format). It is the same function that `git::write_tree` uses internally.
The two OIDs address the same content in different hash spaces.

### 4.4 Translation layer

```
push:
  1. Walk the Fractal tree
  2. For each node, compute git_oid via content_oid()
  3. Write to git object database via git::write_tree()
  4. Create git commit via git::write_commit()
  5. Push via git transport (smart HTTP / SSH)

pull:
  1. Fetch git objects via git transport
  2. Read git tree via git::read_tree()
  3. Reconstruct Fractal tree
  4. Compute native OIDs
  5. Store in FrgmntStore
```

The `GitStore` already implements this bidirectional flow: `write_tree` goes
memory -> git, `read_tree` goes git -> memory. The `flush` / `hydrate` cycle
is the push/pull of the object layer.

---

## 5. What This Enables

### 5.1 mirror CLI operations

| Command | What it does |
|---------|-------------|
| `mirror init` | Creates `.frgmnt/` store. Optionally `git init` alongside for transport. |
| `mirror commit` | Content-addresses current state as a Fractal root. Writes commit with witness. |
| `mirror diff` | Compares two roots structurally. O(log n). Shows `Change::Added/Removed/Modified`. |
| `mirror log` | Traverses commit parent chain. Shows message, witness, timestamp. |
| `mirror push` | Translates Fractal tree to git objects. Pushes via git transport. |
| `mirror pull` | Fetches git objects. Reconstructs Fractal tree. Stores in `.frgmnt/`. |
| `mirror status` | Diffs working state against HEAD. Returns `Imperfect` with `StoreLoss`. |

### 5.2 Git remains the transport layer

Git's value is its transport protocol (smart HTTP, SSH, pack negotiation) and
its ubiquity (every hosting platform speaks git). Fragmentation does not replace
this. It replaces the source of truth:

- The `.frgmnt/` store is the canonical representation.
- The `.git/` directory is a projection for transport.
- `mirror push` translates `.frgmnt/` -> `.git/` -> remote.
- `mirror pull` translates remote -> `.git/` -> `.frgmnt/`.

This is the same relationship as `MirrorOid` (home) and `Sha` (visitor).
The native representation is richer. The git representation is a lossy
projection that enables interop.

### 5.3 The Imperfect pipeline

Every VCS operation returns `Imperfect<T, StoreError, StoreLoss>`:

```rust
let result = store.insert(key, value, size);
// Success: stored, zero loss
// Partial: stored, but dedup_ratio=0.95 (95% was already present)
// Failure: I/O error

let diff_result = diff(&old_root, &new_root);
// The diff itself is lossless, but computing it through the store
// may encounter Partial reads (stale cache entries).
```

The loss does not indicate error. It indicates cost. A store with
`dedup_ratio=0.99` is healthy. A store with `fragmentation_level=0.8`
needs compaction. The loss IS the health metric.

### 5.4 The optic pipeline (with `optics` feature)

```rust
use prism::{apply, Optic};

// Focus on a subtree, project through a transformation, refract back
let beam = Optic::ok((), my_fractal);
let result = apply(&FractalChildPrism::new(target_oid), beam);
// result: Optic<Fractal<D>, Fractal<D>> — the focused subtree
```

The Beam pipeline makes tree navigation compositional. Chain multiple
prisms to navigate deep into a Fractal tree, accumulating loss at each
step. The accumulated loss tells you how far from the root you are and
how much information was shed getting there.

---

## 6. Implementation Phases

### Phase 1: terni dependency + StoreLoss

- Add `terni` as unconditional dependency.
- Define `StoreLoss` implementing `Loss`.
- Define `StoreError` replacing `frgmnt_store::Error`.
- Update `FrgmntStore` signatures to return `Imperfect`.
- Update `Store` (in-memory) to return `Imperfect`.
- All existing tests updated. No behavior change, only richer return types.

### Phase 2: VCS orchestration

- `Head` type with symbolic/detached variants.
- Branch management: create, switch, delete.
- Three-way merge on Fractal trees.
- `mirror init` / `mirror commit` / `mirror diff` / `mirror log` commands.

### Phase 3: Optics (feature-gated)

- Add `prism` as optional dependency behind `optics` feature.
- Implement `FractalDataLens`, `FractalChildPrism`, `FractalTraversal`.
- Wire into the Beam pipeline.
- Formalize the Singularity-to-optic mapping.

### Phase 4: Transport bridge

- `DualFractal` with lazy git OID computation.
- `mirror push` / `mirror pull` using `GitStore` flush/hydrate.
- Pack negotiation via `git2` transport.

---

## 7. Non-Goals

- Replacing git. Git is the transport layer. We are not building a new transport.
- Filesystem semantics. Fragmentation trees are not filesystem trees. The
  `write_tree_named` variant exists for filesystem projection, but the canonical
  tree uses numbered children.
- Backward compatibility with existing `.git/` repositories. A `.frgmnt/` store
  is a new artifact. Git interop is via translation, not compatibility.
- SHA-256 migration. Git's SHA-256 transition is orthogonal. `HashAlg` is generic.
  When git moves to SHA-256, fragmentation already supports it via the trait.

---

## 8. Open Questions

1. **Pack format.** Git's packfile format uses delta compression (OFS_DELTA,
   REF_DELTA) for efficient storage and transfer. Should `.frgmnt/` adopt a
   similar pack format, or rely on git for packed storage? The bounded store
   with eviction may be sufficient for hot objects, with git packs for cold.

2. **Merge strategy.** The three-way structural merge described in 3.5 handles
   tree-level conflicts. What about data-level conflicts within a single node?
   The `Fractal::Shard` carries opaque `D` data. Merge resolution for `D`
   requires domain knowledge (text merge, AST merge, etc.).

3. **Signing.** The `visibility.rs` module has `Public<K, T>` with signatures.
   The `git.rs` module has `commit_signature` for git GPG signatures. How do
   these compose? A fragmentation commit can be signed with Ed25519 (ssh feature)
   AND carry a git-compatible GPG signature for transport.

4. **Lens nodes in VCS.** A `Fractal::Lens` references external trees by OID.
   In a VCS context, this is a submodule: a pointer to another repository's
   commit. The merge semantics for Lens nodes need definition.
