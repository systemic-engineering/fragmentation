# Audit — `CoincidenceHash<5,5>` precondition for T2

*2026-05-24. Mara. Audit (T2 C1 precondition halt).*

Status: **Halted at C1, case (d).** `CoincidenceHash<5,5>` does not exist in
`prism-core` today. T2's substrate-default-hash wiring is blocked until the
type lands somewhere callable from fragmentation. This audit names what's
present, what's missing, and the two options for unblocking.

## 1. What the spec asks for

`fragmentation/docs/specs/mirror-native-vcs.md` §4.6 + §5 T2 declares:

- The default `HashAlg` for `Commit<N, H>` and `Repo` flips from `Sha` to
  `CoincidenceHash<5,5>`.
- `prism-core` owns the `CoincidenceHash<5,5>` implementation — "The Dirac
  operator machinery already lives there per the eigenboard work." Fragmentation
  imports; doesn't redefine.
- The Merkle combine step (parent hash from child hashes) is short to spec
  (~40 lines) derived from the Dirac operator construction: parent's hash is
  the eigenvalues of D² on the incidence structure assembled from the
  children. Spec destination:
  `prism/docs/specs/coincidence-hash-merkle.md`.

Fragmentation's `HashAlg` (the trait it needs the type to satisfy) lives at
`fragmentation/src/sha.rs`:

```rust
pub trait HashAlg: Clone + fmt::Debug + PartialEq + Eq + hash::Hash {
    fn hash(data: &[u8]) -> Self;
    fn from_hex(hex: impl Into<String>) -> Self;
    fn as_str(&self) -> &str;
}
```

Three methods. Flat-bytes-in, self-out. String-typed hex view. No streaming.
No native byte-array shape. No Merkle combine on the trait.

## 2. What prism-core actually has

File-by-file walk of `/Users/reed/dev/projects/prism/core/src/`. Looking for
`CoincidenceHash`, `HashAlg`, or anything that looks like a Merkle combine
over hashes.

### 2.1 `coincidence.rs` — the eigenvalue hash mechanism

What exists:

```rust
pub struct Detector<const N: usize> {
    projections: Vec<Projection>,
    space: String,
}

impl<const N: usize> Detector<N> {
    pub fn canonical(space: impl Into<String>, dimension: usize) -> Self;
    fn detect(&self, data: &[u8]) -> DetectionResult;  // private
    pub fn to_metal(&self) -> crate::metal::MetalPrism;
}

pub trait HashPrism {
    type Input: ?Sized;
    type Output;
    fn review(&self, input: &Self::Input) -> Self::Output;
    fn preview(&self, _output: &Self::Output) -> Option<&Self::Input>;
}

impl<const N: usize> HashPrism for Detector<N> {
    type Input = [u8];
    type Output = String;  // hex string, post-SHA-256 compression
    fn review(&self, input: &[u8]) -> String { ... }
}

pub fn canonical_hash(bytes: &[u8]) -> String;          // Detector<3>, dim=16
pub fn coincidence_hash() -> Named<Detector<3>>;        // named optic
```

What doesn't exist:

- **No `CoincidenceHash` type.** The name appears only in doc comments
  (`oid.rs:18`, `named.rs:248`, `oid.rs` test pin) as a description of
  `Detector<3>`'s behavior. No `struct CoincidenceHash`, no `type
  CoincidenceHash`, no `enum CoincidenceHash`. Verified via
  `pub\s+(struct|type|enum)\s+CoincidenceHash` regex over
  `core/src/**/*.rs` — zero matches.
- **No second const generic.** `Detector` is `Detector<const N: usize>`,
  not `Detector<const N: usize, const M: usize>`. The spec's `<5, 5>`
  pair (mirror's five optics × five detector dimensions, per the
  eigenboard work) has nowhere to land. The closest analog is
  `Detector<5>` with an explicit `dimension=5` argument to `canonical()`,
  but that's runtime data, not type-level.
- **No `HashAlg` impl for `Detector<N>`** (or anything else in prism-core).
  The `HashPrism` trait exists, but it's `review(&[u8]) -> String`, not
  fragmentation's `(hash, from_hex, as_str)` triple. No `from_hex`
  constructor (the canonical detector hash is one-way through SHA-256
  compression). No fixed-size byte output (the output is hex `String`).
- **No Merkle combine step.** `merkle.rs` defines the `MerkleTree` trait
  (`data()`/`children()`) and `diff()`, but the parent-OID-from-children
  computation is *delegated to the caller* via `Addressable::oid()` — every
  consumer rolls its own concatenate-and-hash. There is no
  `combine_children(parent_data: &[u8], child_oids: &[Oid]) -> Oid` on
  the trait. The Dirac-spectrum recursion is not implemented anywhere.

### 2.2 `oid.rs` — `Oid` uses the canonical hash

`Oid::hash(bytes)` delegates to `coincidence::canonical_hash(bytes)` which is
`Detector<3>` with `dimension=16` in the `"content"` space, then SHA-256
compressed to 64-char hex. This is the in-tree `CoincidenceHash<3>` referenced
in the docstring. Not `<5,5>`.

Falls back to `SHA-256(b"prism-core:dark:" || bytes)` for degenerate input
(empty bytes or zero state vector).

Pinned test value: `Oid::hash(b"prism")` =
`08f8e91d230c49a5072202e4e82db8306e226d83f77aa6f57d05dc87b56efc1e` (per
`oid.rs:240`). Stable across versions — any change to the hash function or
its parameters breaks this.

### 2.3 `merkle.rs` — `MerkleTree` trait, no combine

```rust
pub trait MerkleTree: Addressable + Clone {
    type Data: PartialEq;
    fn data(&self) -> &Self::Data;
    fn children(&self) -> &[Self];
    fn is_leaf(&self) -> bool;
    fn degree(&self) -> usize;
}
```

No `combine`. The Addressable contract lives outside the trait; impls
construct their own (per the `TestNode` example in tests, which concatenates
`name` + `:{child_oid}` strings and SHA-hashes).

Fragmentation's `Fractal::content_oid` does the same thing today —
git-format `tree`/`blob` framing + SHA-1. The Dirac-operator recursion has
never been wired anywhere.

### 2.4 `Cargo.toml`

```
dependencies: terni, prism-derive, sha2, hex
```

No feature gate for coincidence — it's always on. So whatever lands won't
need a new feature flag; the existing surface stays.

### 2.5 Pre-existing intent — `coincidence-hash-integration.md`

There is a 2026-04-14 Mara-authored proposal at
`/Users/reed/dev/projects/prism/docs/specs/coincidence-hash-integration.md`
that:

- Names the same problem ("three crates define content addressing
  independently").
- Proposed `CoincidenceHash<N>` as a `HashAlg` impl in fragmentation that
  wrapped `Detector<N>` from prism-core, with `N=3` as the canonical default.
- Was partially executed — the Detector machinery landed in prism-core (per
  Phase 1); `Oid::hash` was rewired to coincidence (per Phase 1); the
  `Addressable` / `Store` / `HashAlg`-for-`Oid` bridges (Phase 2) **were
  never landed**. The bridge crate `coincidence` is still the home for the
  `CoincidenceHash<N>: HashAlg` impl per that spec (`hash.rs` in its
  module list). I haven't audited the `coincidence` crate's current state
  this tick, but the spec language is pre-`<5,5>` — it assumes a single
  const generic `N`, not the matrix shape.

**The `<5,5>` shape is new to the May 24 mirror-native-vcs spec.** It comes
from the eigenboard / Dirac-operator work in
`/Users/reed/dev/systemic.engineering/practice/insights/spectral-db/dirac-operator-on-graphs.md`
but has not been crystallized into a prism-core type. The April spec proposed
landing `CoincidenceHash<N>` (single generic); the May spec assumes
`CoincidenceHash<5,5>` (matrix); neither shape exists in source today.

## 3. Decision tree — case (d) confirmed

From the C1 step-1 decision tree:

- (a) `CoincidenceHash<5,5>` exists ✓ `HashAlg` ✓ combine — **NO** (no
  CoincidenceHash type at all).
- (b) Type ✓ `HashAlg` ✓, combine ✗ — **NO** (no type).
- (c) Type ✓, no `HashAlg` impl — **NO** (no type; closest analog
  `Detector<N>` doesn't implement `HashAlg`-shaped trait either).
- (d) **Type doesn't exist.** ✓

All three gates fail. The substrate has the *machinery* (Detector,
encoding, projection, eigenvalue trace) but not the *type* — no name, no
trait alignment, no two-axis const generic, no parent-from-children operator.

## 4. The blocker, concretely

Three things have to exist before T2 C1 step 2 can run:

1. **A type named `CoincidenceHash<const N: usize, const M: usize>`** (or a
   spec-explicit decision that the second const generic isn't required, in
   which case the spec wording changes to `<5>`). The shape matters because
   the May 24 spec's wiring (`fragmentation/src/sha.rs`: `impl HashAlg for
   CoincidenceHash<5, 5>`) refers to the matrix shape by name.

2. **A `HashAlg`-shaped surface on that type**. Fragmentation's
   `HashAlg` requires `hash(&[u8]) -> Self`, `from_hex(impl Into<String>) ->
   Self`, `as_str(&self) -> &str`, plus `Clone + Debug + PartialEq + Eq +
   Hash`. The current `Detector<N>` produces a one-way hex `String`; there
   is no `from_hex` on either Detector or the eigenvalue output. The
   eigenvalue would need a typed wrapper (e.g.
   `CoincidenceHash([u8; 32])` storing the SHA-256-compressed eigenvalue
   bytes) so it can roundtrip through `from_hex`/`as_str` and equality.

3. **A Merkle combine step** for tree hashes. Today every consumer (prism's
   `TestNode`, fragmentation's `content_oid`) concatenates child OIDs into
   a byte buffer and re-hashes — which works for SHA but throws away the
   Dirac structure. The spec promises the combine is derivable from the
   incidence-matrix construction in §3 of the Dirac insight doc (children
   become edges in a higher-order incidence structure; parent D² has the
   children's eigenvalues as a sub-spectrum). Without this, `CoincidenceHash`
   for tree nodes degenerates into SHA-of-concatenated-children-with-extra-steps
   — the whole point of "hash IS the operator" is lost.

## 5. Two options for unblocking

### Option A — write the precondition in prism-core (Reed's prior intent)

The April `coincidence-hash-integration.md` spec already names this as the
right shape: hash primitives belong in prism-core, fragmentation depends on
prism-core. The May 24 mirror-native-vcs spec §4.6 reaffirms:
*"The Dirac operator machinery already lives there per the eigenboard work."*
(Note: the *machinery* does — `Detector`, `Projection`, encoding — but the
*named type* doesn't yet.)

Work required in prism-core:

- Add `pub struct CoincidenceHash<const N: usize, const M: usize>([u8; 32])`
  (or whatever fixed byte width the eigenvalue compression settles on).
- Add an internal `Detector<N, M>` (two-axis) or a free function
  `coincidence_hash::<N, M>(bytes: &[u8]) -> CoincidenceHash<N, M>` that
  the `(5, 5)` instantiation routes through. The `<5, 5>` meaning per the
  eigenboard: 5 detectors × 5-dimensional vector space, matching the five
  optics (focus, project, refract, split, zoom).
- Define a trait the fragmentation `HashAlg` can shadow (or just let
  fragmentation `impl HashAlg for CoincidenceHash<5, 5>` directly).
- Spec + implement the Merkle combine — the Dirac recursion from §3 of the
  insight doc. The ~40 lines is for the spec; the impl is more.

This is a **separate tick**, not part of T2. Estimate: medium (~1
session). Touches prism-core (which is upstream of fragmentation and
spectral-db; the hash bytes change for every `Oid::hash` caller, the existing
pinned test value at `oid.rs:240` breaks, every prism-core consumer rebakes).

Gains: the architectural story holds. `prism-core` owns the operator;
`fragmentation` consumes; the rest follows. No new home for substrate math
in a downstream crate.

Costs: the pinned `Oid::hash(b"prism")` value moves. Every spec / smoke OID
in the prism + fragmentation + mirror + spectral-db trees that references a
specific hash value rebakes. Per the spec's own note (§4.6, "The boot corpus
rebakes once"): this was already on the table for T5, but it lands earlier
in Option A — at the precondition tick.

### Option B — define `CoincidenceHash<5,5>` locally in fragmentation as a stopgap

Work required in fragmentation:

- New module `fragmentation/src/coincidence.rs` with a thin
  `CoincidenceHash<const N: usize, const M: usize>` wrapper. Internally
  delegates `hash(bytes)` to prism-core's `Detector<N>` (with `dimension=M`
  passed at construction; the second const generic isn't used at the math
  level, only as a type-level marker).
- `impl HashAlg for CoincidenceHash<5, 5>` directly in fragmentation. T2 C1
  step 2 proceeds as written.
- The Merkle combine remains undefined; fragmentation falls back to
  `content_oid` (concat-bytes-then-hash) the way it does today. The spec
  promise of "hash IS the operator" doesn't ship in T2; it becomes
  spec-only until a later tick wires the real combine.

Gains: T2 unblocks immediately. The substrate default flips in this tick,
as promised. Mirror's F-2 (T5) can consume the new default without waiting
on a prism-core change.

Costs: a stopgap by name. The Dirac-as-Merkle-combine claim doesn't ship
with T2 — it ships as a type whose `hash(bytes)` does the right thing for
flat content but whose tree shape is no different from SHA's. The hash-IS-
operator claim has a one-tick gap between "name landed" and "name means what
it claims." When prism-core eventually grows the operator, the type moves
upstream and downstream consumers have a one-rename migration.

A second risk: defining `CoincidenceHash` in fragmentation means
`prism-core::Oid::hash` (which uses `Detector<3>` directly, not via
`CoincidenceHash`) keeps its current bytes, while fragmentation's
`CoincidenceHash<5,5>` produces different bytes for the same input. Two
flavors of "coincidence hash" in the tree, on the same Detector machinery,
with different parameters. Until consolidation, every reader has to ask:
"which CoincidenceHash?"

## 6. Recommendation

**Option A — write the precondition in prism-core, as a separate tick
before T2 resumes.**

Reasoning:

- The May 24 spec §4.6 names prism-core as the home explicitly. Building
  it in fragmentation would contradict the spec one tick after writing it.
- The `<5,5>` matrix shape is the eigenboard's structural claim about how
  the operator IS the hash. Defining the type next to where the math lives
  keeps the substrate math and the substrate identity in one place. Defining
  it in fragmentation puts the math in the consumer.
- Two `CoincidenceHash`-named things on the same `Detector` substrate is
  worse than one delayed tick. The hash-IS-the-operator claim is the whole
  spectral-triple architecture being load-bearing; weakening it once means
  weakening it forever.
- The boot-corpus rebake (the only real cost of Option A) is going to
  happen anyway per the spec's own §4.6 note. Doing it now vs. at T5 is a
  scheduling question, not a structural one.

What the precondition tick should include:

1. `prism-core::CoincidenceHash<const N: usize, const M: usize>` — a typed
   fixed-byte-width wrapper (e.g. `[u8; 32]`) over the eigenvalue, with
   `from_hex` / `as_str` / `Clone+Debug+PartialEq+Eq+Hash`. Either expose
   a `prism-core` trait fragmentation can adapt to its `HashAlg`, or let
   fragmentation `impl HashAlg` on the prism-core type directly (this
   crosses the orphan rule only if we route through a newtype, but the
   newtype can live in fragmentation if needed).
2. The Merkle combine step. Implementation = the recursion in §3 of the
   Dirac insight doc; spec = the ~40 lines promised at
   `prism/docs/specs/coincidence-hash-merkle.md`. Spec lands as part of
   that tick, not separately.
3. Rewire `prism-core::Oid::hash` from `canonical_hash` (Detector<3>) to
   `CoincidenceHash<5,5>` — single hash function across the substrate.
   Update the pinned test value at `oid.rs:240`. Update every consumer's
   smoke OIDs in one sweep.
4. Test: same `[u8]` input yields the same `CoincidenceHash<5,5>` bytes
   across runs; differs from `Sha::hash` bytes; the combine step is
   deterministic and produces output that depends on child order if-and-
   only-if the spec says it should (anti-symmetric vs. symmetric — the
   Dirac doc's signed-incidence-matrix construction has an arbitrary
   orientation choice, which fragmentation already canonicalizes
   lower-OID-first per `coincidence::edge_key`).

T2 then resumes from a clean precondition and lands C1/C2/C3 in one
branch, as the original spec ordering implied.

## 7. Open thread for Alex / Reed

The primary question is the recommendation above (prism-core vs.
fragmentation-local).

A secondary question, if Option A is chosen: who writes the precondition
tick? It's in prism-core, which the T2 plan said *I'm not modifying.* If
that changes, I can write it; if not, this halts until a prism-core-shaped
agent picks it up.

A tertiary question: the `<5, 5>` semantics. The eigenboard's claim is
5 detectors × 5-dimensional vector space matches the five optics. That ties
the hash shape to the substrate's operator count. But mirror's `Lift`
dispatch is six-function (per `mirror-store.md` §3 — `boot/fetch/keys/from/
invalidate/lifecycle`), and spectral-db's surface count is different again.
Is `<5, 5>` the substrate-physics number that doesn't depend on the
consumer's surface count? (Reading the eigenboard work as: yes, the five is
the optic count of the *substrate*, not of any individual consumer's surface.
Confirming.)

---

*Halted at C1. T2 C2 + C3 do not proceed in this tick.*
*No fragmentation source touched.*
*Branch `mara/vcs-substrate` is unchanged from `90d7bb7` (T1's last commit).*
