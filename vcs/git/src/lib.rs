//! Git transport layer for fragmentation content store.
//!
//! Provides git-backed implementations that extend the core `fragmentation` crate:
//!
//! - [`bounded_store::GitBoundedStore`] — byte-bounded cache with git eviction/persistence
//! - [`store::GitStore`] — two-tier store (memory + git ODB)
//! - [`git`] — low-level git read/write operations (write_tree, read_node, etc.)
//! - [`concurrent_store::ConcurrentStoreGitExt`] — flush/hydrate extension for ConcurrentStore
//! - [`commit::DraftWriteExt`] — git-native commit write extension for Draft<Fractal<E>>
//! - [`fuse`] — FUSE filesystem portal (behind the `fuse` / `fuse-mount` features)
//! - `frgmt-git` binary — the CLI (behind the `cli` feature)
//!
//! ## Provenance
//!
//! Folded in 2026-05-24 from the standalone `../fragmentation-git/` repo
//! (commit `f1e1135` extraction) per `docs/specs/mirror-native-vcs.md` §4.5.
//! C3 of T1 then moved `git.rs`, `fuse.rs`, and the CLI binary out of
//! `fragmentation/src/` and into this crate so the substrate carries no
//! `git2` import.

pub mod atomic;
pub mod bounded_store;
pub mod commit;
pub mod concurrent_store;
#[cfg(any(feature = "fuse", feature = "fuse-mount"))]
pub mod fuse;
pub mod git;
pub mod namespaced;
pub mod notes;
pub mod store;
pub mod walk;
