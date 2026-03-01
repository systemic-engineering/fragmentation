# fragmentation

Content-addressed, arbitrary-depth, circular-reflexive trees. Reality for git.

[![Package Version](https://img.shields.io/hexpm/v/fragmentation)](https://hex.pm/packages/fragmentation)
[![Hex Docs](https://img.shields.io/badge/hex-docs-ffaff3)](https://hexdocs.pm/fragmentation/)

```sh
gleam add fragmentation@1
```

## What This Is

A Gleam library for building trees where every node knows its own address, carries a witness record of who observed it, and contains its children directly. Two node types: `Shard` (terminal) and `Fragment` (recursive). Four fields on every witness: author, committer, timestamp, message -- the same four fields as a git commit.

Different witness, different hash. The observation is part of the content.

```gleam
import fragmentation

let witness = fragmentation.witnessed(
  fragmentation.author("alex"),
  fragmentation.committer("reed"),
  fragmentation.timestamp("2026-03-01T00:00:00Z"),
  fragmentation.message("initial"),
)
let leaf = fragmentation.shard(
  fragmentation.ref(fragmentation.hash("leaf-data"), "self"),
  witness,
  "leaf-data",
)
let root = fragmentation.fragment(
  fragmentation.ref(fragmentation.hash("root"), "self"),
  witness,
  "root",
  [leaf],
)

// Content-addressed: same content = same hash
fragmentation.hash_fragment(root)
```

## Modules

| Module | Purpose |
|--------|---------|
| `fragmentation` | Core types, construction, hashing, queries |
| `fragmentation/store` | Content-addressed in-memory storage (Sha -> Fragment) |
| `fragmentation/walk` | Depth-first traversal, fold, find, depth |
| `fragmentation/diff` | Structural comparison between trees |

## Documentation

See [`docs/`](docs/INDEX.md) for the full documentation, including:
- What fragmentation is and why these types
- Why the witness record changes the hash
- How the modules compose
- A guide for agents building on this library

## Development

```sh
gleam test  # 62 tests
```

## Licence

`LICENSE.md` contains the Apache-2.0 licence required by Hex. The actual
governing terms are the [Systemic Engineering License v1.0](REAL_LICENSE.md).
