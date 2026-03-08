# fragmentation

Content-addressed, arbitrary-depth, circular-reflexive trees.

[![CI](https://github.com/systemic-engineering/fragmentation/actions/workflows/test.yml/badge.svg)](https://github.com/systemic-engineering/fragmentation/actions/workflows/test.yml)

```toml
[dependencies]
fragmentation = { git = "https://github.com/systemic-engineering/fragmentation" }
```

## What This Is

A Rust library for building trees where every node is identified by its content. Two node types: `Shard` (terminal) and `Fractal` (recursive). Git-compatible SHA-1 hashing. The observer is part of the commit, not the hash.

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
