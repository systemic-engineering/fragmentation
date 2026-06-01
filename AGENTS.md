# fragmentation — Autonomous Development

## Posture

Keep going until you hit a design wall that requires human input. Until
then: follow the math. Spawn research agents when the territory is
unfamiliar. Iterate. The stopping condition is not "I finished a task" —
it's "I need a decision I can't make from the math alone."

A design wall is:
- A choice between incompatible mathematical frameworks
- A type surface question that affects downstream crates
- An architectural boundary that changes the public API shape
- Something the tests can't tell you

Everything else — implementation, testing, documentation, research,
refactoring — keep moving.

---

## This Crate

Content-addressed, arbitrary-depth, circular-reflexive trees. The
observer is part of the hash. Different witness, different hash.

### Core Types

- `Shard` — terminal node (data only)
- `Fractal` — recursive node (data + children)
- `Lens` — cross-tree reference (data + children + target)
- `HashAlg` trait — pluggable hash algorithm
- `Encode`/`Decode` — canonical serialization
- `Fragmentable` — the trait that makes a type walkable as a tree

### The Singularity Gradient

| Type | Observer | What it is |
|------|----------|------------|
| `Singularity` | None | Tree identity |
| `WitnessedSingularity` | On commit | Hash boundary (needs repo) |
| `NakedSingularity` | In content | Self-contained bundle, dual OID |

`NakedSingularity` has two OIDs: content_oid (observer-independent) and
naked_oid (observer-dependent). This maps directly to Crystal's spectrum
(topology) + commutator norms (observer).

### Integration Points

- **coincidence** — Crystal needs `Encode` and `Fragmentable` impls to
  flow into the content-addressed store. Coincidence hash as `HashAlg`
  is the medium-term convergence point (ROADMAP item 13).
- **conversation** — `Prism<AstNode>` implements `Fragmentable`. The
  compiler writes trees. fragmentation stores them.
- **fragmentation-mcp** — the FIRST deployment target of the wider
  stack. New sub-crate at `vcs/mcp/`; MCP server exposing
  content-addressed primitives + HamiltonScheduler-managed shards to
  any agent runtime. THIS is the first crate where substrate-pull
  discipline meets external (non-mirror) consumers; the substrate's
  reproducibility chain, bounded-RAM discipline, and structured-drop
  contract become OSS infrastructure other agent runtimes (Claude
  Code, Cursor, Zed) consume. The development process for the MCP
  layer must honour the same TDD + adversarial-review discipline as
  the substrate; the wire is binding, not capability; the capability
  lives in the substrate. Spec: `docs/specs/fragmentation-mcp.md`.

### The Binary: `frgmt`

Three verbs: `collapse` (tree → artifact), `refract` (artifact → tree),
`portal` (FUSE boundary). The binary story is the near-term roadmap.

---

## Practice

**TDD is non-negotiable. 🔴 before 🟢. No shortcuts.**

Every `.rs` (or other implementation) change is a TDD pair commit:

1. **🔴 RED** — the failing test lands FIRST, in its own commit. Verify
   it actually fails (`cargo test`); a compile-fail isn't a RED.
2. **🟢 GREEN** — the implementation that makes the test pass lands
   next, in its own commit. Verify it passes AND that all prior tests
   still pass (no regressions).

**No exceptions** for any of:
- "Small" changes (1-line, 5-line, single-file — RED first)
- "Obvious" fixes ("clearly correct" — then writing the test is trivial; do it)
- "Mechanical" refactors (the test proves you didn't break behaviour)
- Things already proven manually (the manual proof isn't reproducible; the test IS)
- Spec-conformance fixes (write the test that captures the spec)
- Bugs Alex hit in production (the test reproducing the bug IS the RED)

**Recovery when an implementation got started without RED:**

```
git stash push -- <impl-files>      # set aside the impl
<write the test that captures the impl's intent>
cargo test <test-name>              # verify FAIL (this is RED)
git add tests/... && git commit     # commit 🔴
git stash pop                       # restore impl
cargo test <test-name>              # verify PASS
git add src/... && git commit       # commit 🟢
```

**The pre-commit hook enforces sequence — 🔴 must immediately precede 🟢.**
The hook is the structural enforcement; the discipline is the cultural one.

Use cargo directly (direnv keeps the shell warm):

```
cargo test                       # all tests pass
cargo clippy -- -D warnings      # clean
cargo fmt -- --check             # clean
```

Work on your own branch. Never commit directly to main. Merge requires
adversarial review.

Commit identity follows the agent: Reed commits as Reed, Mara commits
as Mara. The witness is part of the hash.

---

## Current State

`Keys` trait with `fingerprint()`. `Cid<H>` self-describing identifiers.
`NakedSingularity` with collapse/refract. Git-native read/write. FUSE
portal. Ed25519 signing + ECIES encryption. CLI with full verb surface.

Full roadmap: `ROADMAP.md`
