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
//! | Feature        | Default | What it enables |
//! |---------------|---------|------------------|
//! | `concurrent`  | yes     | `concurrent_store` (DashMap-backed) — Cut 1 of T1 |
//! | `prism-bridge`| yes     | `prism_bridge` — MerkleTree/Store/Loss impls for prism_core |
//! | `visibility`  | yes     | `visibility` — Public/Protected/Private wrappers |
//! | `singularity` | yes     | `naked` + `singularity` — file-system materialization |
//! | `project`     | yes     | `project` + `manifest` — Lens projection |
//! | `supervision` | yes     | `supervision` — ChildSpec / Supervisor / RestartStrategy |
//! | `ssh`         | no      | Ed25519 signing + ECIES encryption (X25519, ChaCha20-Poly1305) |
//! | `gpg`         | no      | GPG signing + encryption via subprocess |
//!
//! All `concurrent`, `prism-bridge`, `visibility`, `singularity`, `project`,
//! `supervision` are default-on for now (existing consumers depend on them).
//! T2/T3 callers should explicitly opt into only what they need; mirror's
//! Layer-1 store uses `--no-default-features` once F-2 lands.

#[cfg(feature = "concurrent")]
pub mod bounded_store;
pub mod cid;
pub mod commit;
#[cfg(feature = "concurrent")]
pub mod concurrent_store;
pub mod diff;
pub mod encoding;
pub mod fragment;
#[cfg(feature = "concurrent")]
pub mod frgmnt_store;
pub mod keys;
#[cfg(feature = "project")]
pub mod manifest;
#[cfg(feature = "singularity")]
pub mod naked;
#[cfg(feature = "prism-bridge")]
pub mod prism_bridge;
#[cfg(feature = "project")]
pub mod project;
pub mod ref_;
pub mod repo;
pub mod sha;
#[cfg(feature = "singularity")]
pub mod singularity;
pub mod spectral_coordinate;
pub mod store;
#[cfg(feature = "supervision")]
pub mod supervision;
#[cfg(feature = "visibility")]
pub mod visibility;
pub mod walk;
pub mod witnessed;
