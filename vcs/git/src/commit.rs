//! Git-native commit write extension for Draft<Fractal<E>>.

use fragmentation::commit::{Commit, Draft};
use fragmentation::encoding::Encode;
use fragmentation::fragment::Fractal;
use fragmentation::sha::Sha;
use fragmentation::witnessed::{Author, Committer, Timestamp, Witnessed};

/// Write a commit with an arbitrary number of parents.
///
/// `parents` may be empty (root commit), one OID (linear), or several OIDs
/// (merge / crystal). Duplicate parent OIDs are rejected by libgit2 and
/// surface as a `git2::Error`; callers should dedupe before calling.
///
/// Phase 4 of `docs/git-native-graph-plan.md`. Crystals carry the previous
/// HEAD commit and every contributing session ref as structural parents.
pub fn write_commit_with_parents(
    repo: &git2::Repository,
    tree: git2::Oid,
    parents: &[git2::Oid],
    sig: &git2::Signature,
    message: &str,
) -> Result<git2::Oid, git2::Error> {
    let tree_obj = repo.find_tree(tree)?;
    let parent_commits: Vec<git2::Commit<'_>> = parents
        .iter()
        .map(|oid| repo.find_commit(*oid))
        .collect::<Result<_, _>>()?;
    let parent_refs: Vec<&git2::Commit<'_>> = parent_commits.iter().collect();
    repo.commit(None, sig, sig, message, &tree_obj, &parent_refs)
}

/// Git-native write extension for Draft<Fractal<E>>.
pub trait DraftWriteExt<E: Encode> {
    /// Write this draft to a git repository.
    /// Returns a Commit (Root or Child) with SHA and witnessed metadata.
    fn write_to_git(
        self,
        repo: &git2::Repository,
        committer: Committer,
    ) -> Result<Commit<Fractal<E>>, git2::Error>;
}

impl<E: Encode> DraftWriteExt<E> for Draft<Fractal<E>> {
    fn write_to_git(
        self,
        repo: &git2::Repository,
        committer: Committer,
    ) -> Result<Commit<Fractal<E>>, git2::Error> {
        let author = self
            .author()
            .cloned()
            .unwrap_or_else(|| Author::new(&committer.name, &committer.email));
        let (node, message, parent) = self.into_parts();
        let oid = crate::git::write_commit(
            repo,
            &node,
            &author,
            &committer,
            &message.0,
            parent.as_ref().map(|p| &p.0),
        )?;
        let git_commit = repo.find_commit(oid)?;
        let timestamp = Timestamp(git_commit.time().seconds().to_string());
        let witnessed = Witnessed::new(author, committer, timestamp);
        let sha = Sha(oid.to_string());

        Ok(match parent {
            None => Commit::full_root(node, witnessed, message, sha),
            Some(p) => Commit::full_child(node, witnessed, message, p, sha),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fragmentation::commit::Draft;
    use fragmentation::encoding;
    use fragmentation::store::Store;
    use fragmentation::witnessed::{Author, Committer};

    fn test_fractal() -> Fractal<String> {
        encoding::encode("hello world")
    }

    fn empty_tree(repo: &git2::Repository) -> git2::Oid {
        let tb = repo.treebuilder(None).unwrap();
        tb.write().unwrap()
    }

    fn empty_commit(repo: &git2::Repository, msg: &str, parents: &[git2::Oid]) -> git2::Oid {
        let sig = git2::Signature::now("test", "test@local").unwrap();
        let tree = empty_tree(repo);
        super::write_commit_with_parents(repo, tree, parents, &sig, msg).unwrap()
    }

    #[test]
    fn write_commit_with_parents_supports_zero_one_two_three() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        // Zero parents → root commit.
        let root = empty_commit(&repo, "root", &[]);
        let root_commit = repo.find_commit(root).unwrap();
        assert_eq!(root_commit.parent_count(), 0);

        // One parent → linear.
        let one = empty_commit(&repo, "one", &[root]);
        let one_commit = repo.find_commit(one).unwrap();
        assert_eq!(one_commit.parent_count(), 1);
        assert_eq!(one_commit.parent_id(0).unwrap(), root);

        // Two parents → merge / crystal.
        let alt = empty_commit(&repo, "alt", &[root]);
        let two = empty_commit(&repo, "two", &[one, alt]);
        let two_commit = repo.find_commit(two).unwrap();
        assert_eq!(two_commit.parent_count(), 2);
        assert_eq!(two_commit.parent_id(0).unwrap(), one);
        assert_eq!(two_commit.parent_id(1).unwrap(), alt);

        // Three parents — octopus merge shape.
        let alt2 = empty_commit(&repo, "alt2", &[root]);
        let three = empty_commit(&repo, "three", &[one, alt, alt2]);
        let three_commit = repo.find_commit(three).unwrap();
        assert_eq!(three_commit.parent_count(), 3);
    }

    /// Draft::commit() must produce the same SHA as git2 with matching inputs.
    #[test]
    fn draft_commit_matches_git() {
        let fractal = test_fractal();
        let author = Author::new("Test", "test@test.com");
        let committer = Committer::new("Test", "test@test.com");
        let timestamp = "1234567890 +0000";
        let epoch: i64 = 1234567890;

        // In-memory via Draft::commit()
        let mut store = Store::<Fractal<String>>::new();
        let draft = Draft::root("test commit", fractal.clone()).authored(author);
        let mem_commit = draft.commit(&mut store, committer, timestamp);

        // git2 with matching fixed timestamp
        let tmp = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(tmp.path()).unwrap();
        let tree_oid = crate::git::write_tree(&git_repo, &fractal).unwrap();
        let tree = git_repo.find_tree(tree_oid).unwrap();
        let git_sig =
            git2::Signature::new("Test", "test@test.com", &git2::Time::new(epoch, 0)).unwrap();
        let git_oid = git_repo
            .commit(None, &git_sig, &git_sig, "test commit", &tree, &[])
            .unwrap();

        assert_eq!(mem_commit.sha().0, git_oid.to_string());
    }
}
