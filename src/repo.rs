//! Repository trait for content-addressed storage.
//!
//! `Repo` defines the interface. `Store` (in store.rs) is the in-memory implementation.
//! A future git2 backend would implement the same trait.

use crate::commit::Commit;
use crate::encoding::Encode;
use crate::fragment::Fractal;
use crate::sha::Sha;

/// Content-addressed repository.
///
/// Owned returns — Store clones from HashMaps, a git2 backend would construct fresh.
pub trait Repo {
    type Element: Encode + Clone;

    /// Store all nodes of a fractal tree recursively. Returns the root content OID.
    fn write_tree(&mut self, fractal: &Fractal<Self::Element>) -> String;

    /// Look up a tree/blob by its content OID.
    fn read_tree(&self, oid: &str) -> Option<Fractal<Self::Element>>;

    /// Store a commit.
    fn write_commit(&mut self, commit: Commit<Self::Element>);

    /// Look up a commit by its SHA.
    fn read_commit(&self, sha: &Sha) -> Option<Commit<Self::Element>>;

    /// Point a ref at a commit SHA.
    fn update_ref(&mut self, name: &str, sha: Sha);

    /// Resolve a ref to a commit SHA.
    fn resolve_ref(&self, name: &str) -> Option<Sha>;
}
