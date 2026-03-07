use std::fs;
use std::io;
use std::path::Path;

use crate::fragment::{self, Fragment};

/// Write a fragment to disk under `dir`, named by its content-addressed SHA.
///
/// Computes the SHA via `hash_fragment`, serializes via `serialize`,
/// then writes to `<dir>/<sha>`.
/// Idempotent: writing the same fragment twice produces the same file.
pub fn write(fragment: &Fragment, dir: &str) -> io::Result<()> {
    let sha = fragment::hash_fragment(fragment);
    let content = fragment::serialize(fragment);
    let path = Path::new(dir).join(&sha);
    fs::write(path, content)
}

// ---------------------------------------------------------------------------
// Native git objects (behind "git" feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "git")]
use crate::witnessed::Witnessed;

/// Write a fragment tree to git objects. Returns the root OID.
/// Shard → blob, Fragment → tree with .data + numbered children.
#[cfg(feature = "git")]
pub fn write_tree(_repo: &git2::Repository, _fragment: &Fragment) -> Result<git2::Oid, git2::Error> {
    todo!()
}

/// Write a fragment and commit it. Returns the commit OID.
/// Witnessed fields map to git author/committer. Message is pass-through.
#[cfg(feature = "git")]
pub fn write_commit(
    _repo: &git2::Repository,
    _fragment: &Fragment,
    _witnessed: &Witnessed,
    _message: &str,
    _parent: Option<&git2::Commit>,
) -> Result<git2::Oid, git2::Error> {
    todo!()
}

/// Reconstruct a Fragment from git objects.
/// Blob → Shard, Tree → Fragment. Witness is not recoverable (lives on commit).
#[cfg(feature = "git")]
pub fn read_tree(
    _repo: &git2::Repository,
    _oid: git2::Oid,
) -> Result<Fragment, Box<dyn std::error::Error>> {
    todo!()
}
