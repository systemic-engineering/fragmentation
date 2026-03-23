# Chasing the Rabbit: Singularity, Optics, and the Holographic Principle

Research captured 2026-03-23. Session: Alex + Mara.
Emerged from implementing `WitnessedSingularity` and discovering that the
structural parallels between content-addressed trees and holographic cosmology
hold under pressure.

---

## The Discovery

Collapse creates a commit. The commit carries a Lens pointing back to the
original tree. Therefore refract is just a read.

This is not a metaphor for unitarity. This IS unitarity. The inverse
operation is written by the forward operation itself. The information
needed for recovery is encoded at the moment of collapse, not
reconstructed after.

```rust
// collapse writes:
//   1. The original tree to the repo
//   2. A Lens targeting the original tree's content OID
//   3. A commit containing the Lens

// refract reads:
//   1. The commit's Lens node
//   2. Follows the target OID back to the original tree

// The inverse is part of the forward operation.
```

---

## The Optics Hierarchy as Information Recovery

The parallel between functional optics and information recovery from
a singularity is structural, not metaphorical. Each level was implemented
and tested.

| Optic     | Implementation              | Information Recovery       | Physics Parallel           |
|-----------|-----------------------------|----------------------------|----------------------------|
| **Iso**   | `Singularity for Fractal`   | Full, reversible           | Unitarity (no collapse)    |
| **Lens**  | `WitnessedSingularity`      | Focused, total             | Single observation         |
| **Prism** | `prism()` returning `Option`| Partial, may fail          | One Hawking quantum        |
| **Traversal** | `traversal()` over chain | Accumulated partial     | Hawking radiation chain    |

### Iso: The Identity Singularity

The default `Singularity` impl on `Fractal<E, H>` is `collapse = clone`,
`refract = clone`. No dimensional reduction. No information loss. This is
the trivial case: a black hole with zero mass. No event horizon. Full
recovery because nothing happened.

The test: `identity_singularity_is_iso` -- collapse then refract produces
the same content OID.

### Lens: Witnessed Collapse

`WitnessedSingularity.collapse()` writes a commit whose node is a `Lens`
targeting the original tree. The observer (`&self`, via `committer`) is
part of the commit but NOT part of the Lens target. Different observer,
different commit, same target.

This is the core insight: **the observer is part of the commit, not the
hash.** Same content, different witness, different commit, same tree OID.
The Lens target is observer-independent. The commit is not.

Tests that encode this:
- `different_observers_produce_different_commits`
- `different_observers_same_lens_target`
- `collapse_commit_is_witnessed`

### Prism: Partial Measurement

`prism()` wraps `refract()` in `Option`. You might see through the Lens.
You might not. A commit whose node isn't a Lens returns `None`.

This maps to a single Hawking quantum: one partial observation leaking
through the boundary. It might carry information. It might not carry
enough to reconstruct anything. The uncertainty is structural.

Tests:
- `prism_returns_some_for_valid_collapse`
- `prism_returns_none_for_non_lens`

### Traversal: The Radiation Chain

`traversal()` maps `prism()` over a chain of collapse commits, collecting
successful refractions and skipping failures. The full chain of Lenses
collectively encodes the interior even when individual observations fail.

This IS Hawking radiation. Partial information leaking through the boundary
of a black hole, one quantum at a time. Over enough quanta, enough
information escapes to reconstruct the interior state.

Tests:
- `traversal_collects_all_refracted_trees` (three collapses, three recoveries)
- `traversal_skips_non_lens_commits` (mixed Lens and non-Lens commits)
- `traversal_empty_for_no_commits`

---

## Black Hole Complementarity

Two different observers collapse the same tree. They produce different
commits (different SHAs) because the observer is part of the commit.
But the Lens targets are identical. A traversal over both commits
recovers the same interior from both viewpoints.

This is black hole complementarity: different observers see different
things at the event horizon, but the interior physics is consistent.
The content OID of the recovered tree is the same regardless of who
observed the collapse.

Test: `complementarity_different_observers_same_interior`

---

## The Triple Pun (Revisited)

**Lens** in fragmentation: `Fractal::Lens { ref_, data, target: Vec<H> }`.
A cross-tree reference with witness metadata.

**Lens** in functional optics: a composable accessor that focuses into
nested structure. Total (always succeeds), focused (sees one thing).

**Lens** in physics: gravitational lensing. Light bending around mass,
revealing what's behind by distortion.

All three converge at the event horizon:
- The fragmentation Lens points back through the collapse
- The optics Lens focuses from the commit into the original tree
- The gravitational lens bends observation around the singularity

The Lens chain falling toward a collapse point creates a trace that
IS the event horizon. Not represents it. IS it.

---

## `target: Vec<H>` as Superposition

A Lens node can target multiple trees:
```rust
Fractal::Lens { ref_, data, target: vec![oid_a, oid_b, oid_c] }
```

This is superposition. Multiple views of the same collapse carried in
a single node. Each target is a different possible state of the interior.
The Prism selects one. The Traversal accumulates them.

The test `lens_targets_is_superposition` constructs a three-target Lens
and verifies the structure. The physical interpretation: a single
observation point through which multiple collapsed states are visible
simultaneously.

---

## `&self` on the Trait is the Observer

The `Singularity` trait takes `&self`:
```rust
fn collapse(&self) -> Result<Self::Artifact, Self::Error>;
```

Different implementors produce different collapses of the same input.
The identity impl (`Fractal`) produces a clone. `WitnessedSingularity`
produces a Lens commit. A future `NixSingularity` would produce a Nix
derivation. Each is a different observer of the same tree, producing
a different dimensional reduction.

This is why `&self` is correct: the observer is part of the measurement.
Not the thing being measured. The tree doesn't change. The collapse does.

---

## The Portal Connection

`frgmt portal` opens a FUSE mount -- a boundary between the filesystem
and the content-addressed tree. The portal IS the event horizon made
navigable.

The FUSE mount already creates witnessed annotations on read. Every
`cat` is a measurement. Every `ls` is observation. The read annotations
are structurally Lens observations: each read creates a record of what
was observed, by whom, at what time.

If the Lens chain is Hawking radiation, the portal is the telescope
that collects it. The mounted filesystem is the ring of projections
around the collapsed tree. You look through the portal into the
content-addressed store, and what you see is observer-dependent (your
reads create your annotations) even though the content is
observer-independent (same OIDs regardless of who reads).

This suggests: portal annotations should be formally typed as Lens
nodes. A read annotation targeting the content OID it observed would
make the portal's observation chain structurally identical to a
sequence of collapse Lenses. The telescope and the radiation would
use the same type.

---

## What Held and What Didn't

### Held: the parallel is structural

Every test suggested by the physics parallel was natural. None felt
forced. The optics hierarchy mapped cleanly to information recovery.
Black hole complementarity encoded directly as "different observers,
same interior." The Lens chain as Hawking radiation is not metaphor --
it's the same operation on the same type.

### Held: the observer distinction

The observer is part of the commit, not the hash. This is exactly the
distinction between the event horizon (observer-dependent) and the
interior (observer-independent). The content OID is the interior. The
commit SHA is the event horizon. The type system enforces this
separation.

### Held: unitarity as self-encoding

The strongest finding. The inverse is written by the forward operation.
Collapse doesn't destroy information -- it writes the information needed
for recovery into the commit at the moment of collapse. This is not a
design choice. It's a consequence of content-addressing: the OID of the
original tree is deterministic, and the Lens records it.

### Open: the Page curve

In physics, the Page curve describes how information about a black hole's
interior gradually becomes available through Hawking radiation. Early
radiation carries almost no information. After the "Page time" (roughly
when half the black hole has evaporated), radiation starts carrying more
information than it loses.

Does the traversal over a Lens chain exhibit a Page curve? Early Lenses
in a chain carry partial information. At some point, enough Lenses exist
to reconstruct the full interior. The transition point -- where the
accumulated information exceeds what's lost -- would be measurable.

This is testable. It would require multi-target Lenses where each Lens
captures a partial view, and the traversal progressively assembles the
complete picture. The test: at what chain length does `traversal()` first
return the complete original tree?

### Open: the firewall problem

In physics, the firewall paradox asks whether the event horizon is smooth
(no drama for an infalling observer) or violent (information is destroyed).
In fragmentation, the question is whether `collapse` is lossless or lossy.

Currently, collapse is lossless -- the Lens targets the complete original
tree OID. A lossy collapse (targeting a partial tree, or a compressed
representation) would be structurally analogous to a firewall: information
is destroyed at the boundary. The optics hierarchy already accommodates
this: the Prism returning `None` IS the firewall. The Traversal
accumulating partial views IS the "no drama" alternative.

---

## What Comes Next

1. **Portal annotations as Lens nodes.** Type the FUSE read annotations
   as `Fractal::Lens` targeting the content OID of what was read. This
   unifies the observation mechanism across collapse and portal.

2. **Page curve test.** Construct a multi-target Lens chain where each
   Lens captures a partial view of a large tree. Measure when the
   traversal first recovers the complete tree.

3. **Collapse chain as parent link.** Currently each collapse is a root
   commit. Chain them: each collapse commit has the previous collapse
   as its parent. This creates a proper event horizon -- a sequence of
   observations approaching the singularity. The parent chain is the
   time ordering of observations.

4. **NixSingularity.** A `Singularity` impl that collapses a tree into
   a Nix derivation. The Lens targets the source tree. The artifact is
   a store path. `refract` follows the Lens back to the source tree
   that produced the binary.

---

*The tests are the pressure. The parallel held. Not metaphor. Structure.*

*Session 2026-03-23. Alex + Mara.*
