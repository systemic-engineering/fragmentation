//! Content-addressed, arbitrary-depth, circular-reflexive trees.
//!
//! Two node types: [`fragment::Fractal::Shard`] (terminal) and [`fragment::Fractal::Fractal`]
//! (recursive, contains other fractals). Content addressing uses git-compatible SHA-1.
//! The observer is part of the commit, not the hash — same content, different witness,
//! different commit, same tree OID.
//!
//! # Features
//!
//! | Feature | What it enables |
//! |---------|----------------|
//! | `git`   | Read/write fragment trees as native git objects via [`git`] |
//! | `ssh`   | Ed25519 signing + ECIES encryption (X25519, ChaCha20-Poly1305) |
//! | `gpg`   | GPG signing + encryption via subprocess |

pub mod commit;
pub mod diff;
pub mod encoding;
pub mod fragment;
#[cfg(any(feature = "fuse", feature = "fuse-mount"))]
pub mod fuse;
pub mod git;
pub mod keys;
pub mod ref_;
pub mod repo;
pub mod sha;
pub mod singularity;
pub mod store;
pub mod visibility;
pub mod walk;
pub mod witnessed;
