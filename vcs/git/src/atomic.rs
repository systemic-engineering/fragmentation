//! Atomic ref updates with compare-and-set semantics.
//!
//! The single primitive `update_ref_atomic` is the only path through which the
//! spectral graph's HEAD ref should advance. It models two operations:
//!
//! - **Create-if-absent** (`expected_old = None`) — succeeds only when the ref
//!   does not yet exist.
//! - **CAS** (`expected_old = Some(oid)`) — succeeds only when the ref
//!   currently points at `oid`.
//!
//! Race-free in the single-writer case (Phase 2 of the git-native graph plan).
//! For multi-writer the same primitive will back the per-session WAL settlement
//! retry loop (see Phase 2.5 / R6).

use git2::{Oid, Reference, Repository};

/// Errors from atomic ref operations.
#[derive(Debug)]
pub enum Error {
    /// libgit2 returned an error.
    Git(git2::Error),
    /// CAS attempt: the ref points at something other than `expected`,
    /// or the ref does not exist at all.
    CasMismatch {
        ref_name: String,
        expected: Oid,
        actual: Option<Oid>,
    },
    /// `expected_old = None` was passed but the ref already exists.
    AlreadyExists { ref_name: String, actual: Oid },
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Error::Git(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Git(e) => write!(f, "git: {}", e),
            Error::CasMismatch {
                ref_name,
                expected,
                actual,
            } => write!(
                f,
                "cas mismatch on {}: expected {}, actual {:?}",
                ref_name, expected, actual
            ),
            Error::AlreadyExists { ref_name, actual } => {
                write!(f, "ref already exists: {} -> {}", ref_name, actual)
            }
        }
    }
}

impl std::error::Error for Error {}

/// Atomically update `ref_name` to point at `new`.
///
/// `expected_old`:
/// - `Some(oid)` — CAS update: succeeds iff the ref currently resolves to `oid`.
/// - `None`      — create-if-absent: succeeds iff the ref does not yet exist.
///
/// Failures are returned as domain errors (`CasMismatch`, `AlreadyExists`); the
/// function does not panic. The reflog message is set to a deterministic
/// `update-ref-atomic: ...` string.
pub fn update_ref_atomic(
    repo: &Repository,
    ref_name: &str,
    expected_old: Option<Oid>,
    new: Oid,
) -> Result<(), Error> {
    match expected_old {
        Some(expected) => {
            // Resolve the ref. If it doesn't exist, that's a CAS mismatch
            // (nothing exists at the address we wanted to compare against).
            let reference: Reference = match repo.find_reference(ref_name) {
                Ok(r) => r,
                Err(_) => {
                    return Err(Error::CasMismatch {
                        ref_name: ref_name.to_string(),
                        expected,
                        actual: None,
                    });
                }
            };
            // Symbolic refs must be resolved to the underlying direct ref before
            // the OID comparison.
            let mut resolved = reference.resolve()?;
            let actual = resolved.target();
            if actual != Some(expected) {
                return Err(Error::CasMismatch {
                    ref_name: ref_name.to_string(),
                    expected,
                    actual,
                });
            }
            let msg = format!("update-ref-atomic: {} -> {}", expected, new);
            resolved.set_target(new, &msg)?;
            Ok(())
        }
        None => {
            if let Ok(existing) = repo.find_reference(ref_name) {
                let actual = existing
                    .resolve()
                    .ok()
                    .and_then(|r| r.target())
                    .unwrap_or_else(Oid::zero);
                return Err(Error::AlreadyExists {
                    ref_name: ref_name.to_string(),
                    actual,
                });
            }
            let msg = format!("update-ref-atomic: create {}", new);
            repo.reference(ref_name, new, false, &msg)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    fn empty_commit(repo: &Repository, message: &str, parent: Option<Oid>) -> Oid {
        let sig = git2::Signature::now("test", "test@local").unwrap();
        let tb = repo.treebuilder(None).unwrap();
        let tree_oid = tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let parents: Vec<git2::Commit> = parent
            .into_iter()
            .map(|p| repo.find_commit(p).unwrap())
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(None, &sig, &sig, message, &tree, &parent_refs)
            .unwrap()
    }

    #[test]
    fn update_ref_atomic_creates_new_ref() {
        let (_dir, repo) = fresh_repo();
        let c1 = empty_commit(&repo, "first", None);

        update_ref_atomic(&repo, "refs/test/foo", None, c1).unwrap();

        let r = repo.find_reference("refs/test/foo").unwrap();
        assert_eq!(r.target(), Some(c1));
    }

    #[test]
    fn update_ref_atomic_cas_succeeds_on_match() {
        let (_dir, repo) = fresh_repo();
        let c1 = empty_commit(&repo, "first", None);
        let c2 = empty_commit(&repo, "second", Some(c1));

        update_ref_atomic(&repo, "refs/test/foo", None, c1).unwrap();
        update_ref_atomic(&repo, "refs/test/foo", Some(c1), c2).unwrap();

        let r = repo.find_reference("refs/test/foo").unwrap();
        assert_eq!(r.target(), Some(c2));
    }

    #[test]
    fn update_ref_atomic_cas_errors_on_mismatch() {
        let (_dir, repo) = fresh_repo();
        let c1 = empty_commit(&repo, "first", None);
        let c2 = empty_commit(&repo, "second", Some(c1));
        let c3 = empty_commit(&repo, "third", Some(c1));

        update_ref_atomic(&repo, "refs/test/foo", None, c1).unwrap();
        let err = update_ref_atomic(&repo, "refs/test/foo", Some(c2), c3).unwrap_err();
        match err {
            Error::CasMismatch {
                expected, actual, ..
            } => {
                assert_eq!(expected, c2);
                assert_eq!(actual, Some(c1));
            }
            other => panic!("expected CasMismatch, got {:?}", other),
        }
        // Ref unchanged.
        let r = repo.find_reference("refs/test/foo").unwrap();
        assert_eq!(r.target(), Some(c1));
    }

    #[test]
    fn update_ref_atomic_already_exists_errors() {
        let (_dir, repo) = fresh_repo();
        let c1 = empty_commit(&repo, "first", None);
        let c2 = empty_commit(&repo, "second", Some(c1));

        update_ref_atomic(&repo, "refs/test/foo", None, c1).unwrap();
        let err = update_ref_atomic(&repo, "refs/test/foo", None, c2).unwrap_err();
        match err {
            Error::AlreadyExists { actual, .. } => assert_eq!(actual, c1),
            other => panic!("expected AlreadyExists, got {:?}", other),
        }
        // Ref unchanged.
        let r = repo.find_reference("refs/test/foo").unwrap();
        assert_eq!(r.target(), Some(c1));
    }
}
