# fragmentation

Content-addressed, arbitrary-depth, circular-reflexive trees.

[![CI](https://github.com/systemic-engineering/fragmentation/actions/workflows/test.yml/badge.svg)](https://github.com/systemic-engineering/fragmentation/actions/workflows/test.yml)

```toml
[dependencies]
fragmentation = { git = "https://github.com/systemic-engineering/fragmentation" }
```

## What This Is

A Rust library for building trees where every node is identified by its
content. Two node types: `Shard` (terminal) and `Fractal` (recursive).
Git-compatible SHA-1 hashing at the wire boundary; content-addressed
`SpectralCoordinate<5>` internally. The observer is part of the commit,
not the hash.

As of 2026-06-01, fragmentation is **also the substrate beneath
fragmentation-mcp** — a standalone Model Context Protocol server
that exposes content-addressed primitives + the HamiltonScheduler's
hot/cold management to any agent runtime (Claude Code, Cursor, Zed,
any MCP-aware client). See [`docs/specs/fragmentation-mcp.md`](docs/specs/fragmentation-mcp.md)
for the architecture; the MCP layer lives in the new
`vcs/mcp/` workspace member. Fragmentation-mcp is the **first deployment
target** of the wider stack — ships standalone, useful without mirror,
open-source infrastructure that fills a real gap in the agent-runtime
ecosystem (existing git-MCPs are CLI wrappers; this one speaks
content-addressed primitives directly).

```rust
use fragmentation::fragment::{Fragment, blob_oid};

// Shard -- terminal node
let leaf = Fragment::shard("hello");

// Fractal -- recursive node
let tree = Fragment::fractal("root", vec![leaf]);

// Content-addressed: same content = same hash
let oid = tree.content_oid();
```

Different witness, different commit. Same content, same tree. The observer changes the record without changing the content.

## DAG-native

Fragmentation is **graph-native DAG** today (v1). Content addressing
handles directed acyclic multi-parent references natively: same content
→ same OID, regardless of how many parents reference it.

- `Fractal::Branch` carries `fractal: Vec<Fractal<E, H>>` by value. The
  same child node, cloned into two parents, collapses to one stored node
  under the content-address.
- `Fractal::Lens` carries `target: Vec<H>` for explicit OID-only
  references (proper graph edges, not containment). Multiple lenses can
  point at the same target without duplication.

**Cycle handling is deferred to spectral-db (v1.5).** spectral-db
exercises actually-cyclic graphs with the witnessed-computation
contract: each cycle traversal records a measurement, the measurement
IS the fixed-point witness, and `kintsugi` ensures the measurement
converges. Fragmentation itself treats cycle detection as an error
class; the v1/v1.5 boundary is named in
`docs/specs/mirror-native-vcs.md` §4.6 (Consequence 3).
## Adapters

Fragmentation ships as a workspace with VCS-adjacent adapter crates:

| Crate | Role |
|---|---|
| `fragmentation` | the substrate; content-addressed primitives, hash trait, store types |
| `fragmentation-git` (`vcs/git/`) | git wire interop; SHA-1 ↔ `SpectralCoordinate<5>` crosswalk; FUSE portal; `frgmt-git` CLI |
| `fragmentation-jj` (`vcs/jj/`) | jj-native backend (planned; per `docs/specs/mirror-native-vcs.md`) |
| `fragmentation-mcp` (`vcs/mcp/`, new) | MCP server; agent-runtime substrate; `fragmentation serve --stdio` |

The substrate compiles dependency-free of all adapters; each adapter
pulls fragmentation as its dependency. `prism_core` stays zero-dep.

Fragmentation-mcp is the **first deployment target** of the wider
stack — ships standalone, useful without mirror, open-source
infrastructure that fills a real gap in the agent-runtime ecosystem
(existing git-MCPs are CLI wrappers; fragmentation-mcp speaks
content-addressed primitives directly, with bounded RAM via the
HamiltonScheduler and structured drops via [[docs/specs/lens-transit]]).
See [`docs/specs/fragmentation-mcp.md`](docs/specs/fragmentation-mcp.md).


## Features

| Feature | What it enables |
|---------|----------------|
| `git`   | Read/write fragment trees as native git objects |
| `ssh`   | Ed25519 signing + ECIES encryption (X25519, ChaCha20-Poly1305) |
| `gpg`   | GPG signing + encryption via subprocess |

## Modules

| Module | Purpose |
|--------|---------|
| `fragment` | `Fragment<E>`, construction, content addressing |
| `ref_` | `Ref` -- content address (SHA + label) |
| `sha` | `Sha` -- SHA-1 and SHA-256 |
| `store` | Content-addressed in-memory storage |
| `walk` | Depth-first traversal, fold, find, depth |
| `diff` | Positional structural comparison |
| `encoding` | Text as five-level fragment trees |
| `witnessed` | Commit metadata -- author, committer, timestamp, message |
| `git` | Fragment persistence to git repositories |
| `keys` | Sign, encrypt, decrypt (SSH, GPG, plain) |
| `actor` | Witness identity with encoding boundary and keys |

## Documentation

See [`docs/`](docs/INDEX.md).

## Development

```sh
cargo test --all-features  # 185 tests
```

## Licence

[systemic.engineering License v1.0](LICENSE.md)
