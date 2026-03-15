//! In-memory git-compatible object store.
//!
//! No git2 dependency. Pure Rust. Content-addressed.
//! Commits produce the same SHAs as git2 with identical inputs.
//! When the backend swaps to disk, hashes remain valid.

use std::collections::HashMap;

use crate::commit::{Commit, Draft, Parent};
use crate::encoding::Encode;
use crate::fragment::{Blob, Fractal};
use crate::sha::Sha;
use crate::witnessed::Committer;

/// In-memory git-compatible object store.
pub struct Repo<E = Blob> {
    objects: HashMap<String, Fractal<E>>,
    commits: HashMap<String, Commit<E>>,
    refs: HashMap<String, Sha>,
}

impl<E: Encode + Clone> Repo<E> {
    /// Empty repo. No objects, no commits, no refs.
    pub fn new() -> Self {
        Repo {
            objects: HashMap::new(),
            commits: HashMap::new(),
            refs: HashMap::new(),
        }
    }

    /// Store all nodes of a fractal tree recursively. Returns the root content OID.
    pub fn write_tree(&mut self, _fractal: &Fractal<E>) -> String {
        todo!()
    }

    /// Look up a tree/blob by its content OID.
    pub fn read_tree(&self, oid: &str) -> Option<&Fractal<E>> {
        self.objects.get(oid)
    }

    /// Create a commit from a draft. Computes git-compatible commit SHA.
    ///
    /// `timestamp` is in git format: "{epoch} {tz_offset}", e.g. "1234567890 +0000".
    pub fn commit(
        &mut self,
        _draft: Draft<E>,
        _committer: Committer,
        _timestamp: &str,
    ) -> Commit<E> {
        todo!()
    }

    /// Look up a commit by its SHA.
    pub fn get_commit(&self, sha: &Sha) -> Option<&Commit<E>> {
        self.commits.get(&sha.0)
    }

    /// Point a ref at a commit SHA.
    pub fn update_ref(&mut self, _name: &str, _sha: Sha) {
        todo!()
    }

    /// Resolve a ref to a commit SHA.
    pub fn resolve_ref(&self, name: &str) -> Option<&Sha> {
        self.refs.get(name)
    }
}

impl<E: Encode + Clone> Default for Repo<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::Draft;
    use crate::encoding;
    use crate::fragment::{blob_oid, Fractal};
    use crate::ref_::Ref;
    use crate::sha::Sha;
    use crate::witnessed::{Author, Committer};

    fn test_shard() -> Fractal<String> {
        let data = "hello";
        let oid = blob_oid(data);
        let r = Ref::new(Sha(oid), "test");
        Fractal::shard(r, data)
    }

    fn test_fractal() -> Fractal<String> {
        encoding::encode("hello world")
    }

    fn test_committer() -> Committer {
        Committer::new("Test", "test@test.com")
    }

    const TEST_TIMESTAMP: &str = "1234567890 +0000";

    #[test]
    fn repo_empty() {
        let repo: Repo<String> = Repo::new();
        assert!(repo.get_commit(&Sha("anything".into())).is_none());
        assert!(repo.resolve_ref("HEAD").is_none());
    }

    #[test]
    fn repo_write_read_tree() {
        let mut repo: Repo<String> = Repo::new();
        let fractal = test_fractal();
        let oid = repo.write_tree(&fractal);
        let read_back = repo.read_tree(&oid).expect("should find tree");
        assert_eq!(read_back, &fractal);
    }

    #[test]
    fn repo_commit_root() {
        let mut repo: Repo<String> = Repo::new();
        let fractal = test_fractal();
        let draft = Draft::root("initial", fractal);
        let commit = repo.commit(draft, test_committer(), TEST_TIMESTAMP);
        assert!(matches!(commit, Commit::Root { .. }));
        assert!(!commit.sha().0.is_empty());
    }

    #[test]
    fn repo_commit_child() {
        let mut repo: Repo<String> = Repo::new();
        let fractal = test_fractal();
        let root_draft = Draft::root("root", fractal.clone());
        let root = repo.commit(root_draft, test_committer(), TEST_TIMESTAMP);
        let child_draft = root.child("child", fractal);
        let child = repo.commit(child_draft, test_committer(), "1234567891 +0000");
        assert!(matches!(child, Commit::Child { .. }));
        assert_ne!(child.sha(), root.sha());
    }

    #[cfg(feature = "git")]
    #[test]
    fn repo_commit_sha_matches_git() {
        let fractal = test_fractal();
        let timestamp = "1234567890 +0000";
        let author = Author::new("Test", "test@test.com");
        let committer = Committer::new("Test", "test@test.com");

        // In-memory
        let mut mem_repo: Repo<String> = Repo::new();
        let draft = Draft::root("test commit", fractal.clone()).authored(author.clone());
        let mem_commit = mem_repo.commit(draft, committer.clone(), timestamp);

        // git2
        let tmp = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(tmp.path()).unwrap();
        let tree_oid = crate::git::write_tree(&git_repo, &fractal).unwrap();
        let tree = git_repo.find_tree(tree_oid).unwrap();
        let epoch: i64 = 1234567890;
        let git_sig =
            git2::Signature::new("Test", "test@test.com", &git2::Time::new(epoch, 0)).unwrap();
        let git_oid = git_repo
            .commit(None, &git_sig, &git_sig, "test commit", &tree, &[])
            .unwrap();

        assert_eq!(mem_commit.sha().0, git_oid.to_string());
    }

    #[test]
    fn repo_refs() {
        let mut repo: Repo<String> = Repo::new();
        let sha = Sha("abc123".into());
        repo.update_ref("refs/heads/main", sha.clone());
        assert_eq!(repo.resolve_ref("refs/heads/main"), Some(&sha));
        assert_eq!(repo.resolve_ref("refs/heads/other"), None);
    }

    #[test]
    fn repo_commit_chain() {
        let mut repo: Repo<String> = Repo::new();
        let fractal = test_fractal();

        let c1 = repo.commit(
            Draft::root("first", fractal.clone()),
            test_committer(),
            "1000000000 +0000",
        );
        let c2 = repo.commit(
            c1.child("second", fractal.clone()),
            test_committer(),
            "1000000001 +0000",
        );
        let c3 = repo.commit(
            c2.child("third", fractal),
            test_committer(),
            "1000000002 +0000",
        );

        assert!(matches!(
            repo.get_commit(c3.sha()),
            Some(Commit::Child { .. })
        ));
        assert!(matches!(
            repo.get_commit(c2.sha()),
            Some(Commit::Child { .. })
        ));
        assert!(matches!(
            repo.get_commit(c1.sha()),
            Some(Commit::Root { .. })
        ));
    }

    #[test]
    fn repo_deduplication() {
        let mut repo: Repo<String> = Repo::new();
        let fractal = test_fractal();
        let oid1 = repo.write_tree(&fractal);
        let oid2 = repo.write_tree(&fractal);
        assert_eq!(oid1, oid2);
    }
}
