//! Tool modules — per-tool dispatch + parameter parsing.
//!
//! Each submodule owns one tool category. T3 lands `content`
//! (`fragmentation.commit` + `fragmentation.read`); T4+ adds
//! `refs`, `history`, `diff`, etc.
//!
//! Substrate-pull: `[substrate-pull:realize]` — each tool module is
//! boundary Rust at the dispatch altitude. The capabilities (storage,
//! hashing, traversal) live in `fragmentation` and `fragmentation-git`;
//! the modules here are thin parse + route shims.

pub mod content;
