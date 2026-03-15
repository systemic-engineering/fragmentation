//! In-memory git-compatible object store.
//!
//! No git2 dependency. Pure Rust. Content-addressed.
//! Commits produce the same SHAs as git2 with identical inputs.
//! When the backend swaps to disk, hashes remain valid.

use std::collections::HashMap;

use crate::commit::{Commit, Draft, Draftable};
use crate::encoding::Encode;
use crate::fragment::{content_oid, tree_oid_bytes, Blob, Fractal};
use crate::sha::Sha;
use crate::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

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
    pub fn write_tree(&mut self, fractal: &Fractal<E>) -> String {
        if let Fractal::Fractal {
            fractal: children, ..
        } = fractal
        {
            for child in children {
                self.write_tree(child);
            }
        }
        let oid = content_oid(fractal);
        self.objects
            .entry(oid.clone())
            .or_insert_with(|| fractal.clone());
        oid
    }

    /// Look up a tree/blob by its content OID.
    pub fn read_tree(&self, oid: &str) -> Option<&Fractal<E>> {
        self.objects.get(oid)
    }

    /// Create a commit from a draft. Computes git-compatible commit SHA.
    ///
    /// `timestamp` is in git format: "{epoch} {tz_offset}", e.g. "1234567890 +0000".
    pub fn commit(&mut self, draft: Draft<E>, committer: Committer, timestamp: &str) -> Commit<E> {
        let author = draft
            .author()
            .cloned()
            .unwrap_or_else(|| Author::new(&committer.name, &committer.email));

        let fractal = draft.fractal().clone();
        let message_str = draft.message().0.clone();
        let parent = draft.parent().cloned();

        // Compute tree OID — shards get wrapped in a tree (matching git::write_commit)
        let tree_oid = match &fractal {
            Fractal::Shard { data, .. } => tree_oid_bytes(&data.encode(), &[] as &[Fractal<E>]),
            Fractal::Fractal { .. } => content_oid(&fractal),
        };

        self.write_tree(&fractal);

        let commit_sha = compute_commit_sha(
            &tree_oid,
            parent.as_ref().map(|p| p.0 .0.as_str()),
            &author,
            &committer,
            timestamp,
            &message_str,
        );

        let sha = Sha(commit_sha);
        let epoch = timestamp.split_whitespace().next().unwrap_or(timestamp);
        let witnessed = Witnessed::new(author, committer, Timestamp(epoch.to_string()));
        let msg = Message(message_str);

        let commit = match parent {
            None => Commit::full_root(fractal, witnessed, msg, sha.clone()),
            Some(p) => Commit::full_child(fractal, witnessed, msg, p, sha.clone()),
        };

        self.commits.insert(sha.0.clone(), commit.clone());
        commit
    }

    /// Look up a commit by its SHA.
    pub fn get_commit(&self, sha: &Sha) -> Option<&Commit<E>> {
        self.commits.get(&sha.0)
    }

    /// Point a ref at a commit SHA.
    pub fn update_ref(&mut self, name: &str, sha: Sha) {
        self.refs.insert(name.to_string(), sha);
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

/// Compute a git-compatible commit SHA from the raw commit object fields.
///
/// Mirrors git's commit object format exactly:
/// ```text
/// tree {tree_oid}\n
/// [parent {parent_oid}\n]
/// author {name} <{email}> {timestamp}\n
/// committer {name} <{email}> {timestamp}\n
/// \n
/// {message}
/// ```
fn compute_commit_sha(
    tree_oid: &str,
    parent_sha: Option<&str>,
    author: &Author,
    committer: &Committer,
    timestamp: &str,
    message: &str,
) -> String {
    use sha1::{Digest, Sha1};

    let mut content = String::new();
    content.push_str(&format!("tree {}\n", tree_oid));
    if let Some(parent) = parent_sha {
        content.push_str(&format!("parent {}\n", parent));
    }
    content.push_str(&format!(
        "author {} <{}> {}\n",
        author.name, author.email, timestamp
    ));
    content.push_str(&format!(
        "committer {} <{}> {}\n",
        committer.name, committer.email, timestamp
    ));
    content.push_str(&format!("\n{}", message));

    let header = format!("commit {}\0", content.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::Draft;
    use crate::encoding;
    use crate::fragment::Fractal;
    use crate::sha::Sha;
    use crate::witnessed::Committer;

    #[cfg(feature = "git")]
    use crate::witnessed::Author;

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
