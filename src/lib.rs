//! Content-addressed, arbitrary-depth, circular-reflexive trees.
//!
//! Two node types: [`fragment::Fractal::Shard`] (terminal) and [`fragment::Fractal::Branch`]
//! (recursive, contains other fractals). Content addressing uses pluggable hash
//! algorithms ([`sha::HashAlg`]). The observer is part of the commit, not the hash —
//! same content, different witness, different commit, same tree OID.
//!
//! # Layering
//!
//! `fragmentation` is the VCS-agnostic content-backed store. It owns the
//! content primitives, the storage backends, and the hash/encoding contracts.
//! VCS adapters live in workspace siblings:
//!
//! - [`fragmentation-git`](https://docs.rs/fragmentation-git) — git interop
//!   (`vcs/git/`). Push/pull/clone, FUSE portal, CLI binary `frgmt-git`.
//! - `fragmentation-jj` — jj-native backend (`vcs/jj/`). Reserved for T4.
//!
//! See `docs/specs/mirror-native-vcs.md` for the full layering.
//!
//! # Features
//!
//! | Feature | What it enables |
//! |---------|----------------|
//! | `ssh`   | Ed25519 signing + ECIES encryption (X25519, ChaCha20-Poly1305) |
//! | `gpg`   | GPG signing + encryption via subprocess |

pub mod bounded_store;
pub mod cid;
pub mod commit;
pub mod concurrent_store;
pub mod diff;
pub mod encoding;
pub mod fragment;
pub mod frgmnt_store;
pub mod keys;
pub mod manifest;
pub mod naked;
pub mod prism_bridge;
pub mod project;
pub mod ref_;
pub mod repo;
pub mod sha;
pub mod singularity;
pub mod store;
pub mod supervision;
pub mod visibility;
pub mod walk;
pub mod witnessed;
