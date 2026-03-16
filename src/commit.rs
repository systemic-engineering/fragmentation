use crate::encoding::Encode;
use crate::fragment::{content_oid, tree_oid_bytes, Fractal};
use crate::repo::Repo;
use crate::sha::Sha;
use crate::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

/// Typed reference to a parent commit. Not a raw SHA — a graph edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parent(pub Sha);

/// The commit graph interface. Draft and Commit both implement this.
/// A signed commit (Public<K, T: Draftable>) also implements it.
pub trait Draftable {
    type Element;
    fn fractal(&self) -> &Fractal<Self::Element>;
    fn message(&self) -> &Message;
    fn parent(&self) -> Option<&Parent>;
}

/// A commit before it has been written to git.
/// Has content and intent, but no SHA and no witnessed metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft<E> {
    fractal: Fractal<E>,
    message: Message,
    parent: Option<Parent>,
    author: Option<Author>,
}

/// A commit that has been written to git.
/// Root has no parent. Child has a parent. The enum discriminant carries the distinction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Commit<E> {
    /// Terminal in the commit graph. No parent.
    Root {
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        sha: Sha,
    },
    /// Has a parent. Always.
    Child {
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        parent: Parent,
        sha: Sha,
    },
}

impl<E> Draft<E> {
    /// Create a root draft (no parent).
    pub fn root(message: impl Into<String>, fractal: Fractal<E>) -> Self {
        Draft {
            fractal,
            message: Message(message.into()),
            parent: None,
            author: None,
        }
    }

    /// Create a draft with a parent.
    pub fn new(message: impl Into<String>, fractal: Fractal<E>, parent: Parent) -> Self {
        Draft {
            fractal,
            message: Message(message.into()),
            parent: Some(parent),
            author: None,
        }
    }

    /// Stamp the Author.
    pub fn authored(mut self, author: Author) -> Self {
        self.author = Some(author);
        self
    }

    /// The author, if set.
    pub fn author(&self) -> Option<&Author> {
        self.author.as_ref()
    }

    /// Commit this draft to a repo. Computes git-compatible commit SHA.
    ///
    /// `timestamp` is in git format: "{epoch} {tz_offset}", e.g. "1234567890 +0000".
    pub fn commit(
        self,
        repo: &mut impl Repo<Element = E>,
        committer: Committer,
        timestamp: &str,
    ) -> Commit<E>
    where
        E: Encode + Clone,
    {
        let author = self
            .author
            .unwrap_or_else(|| Author::new(&committer.name, &committer.email));

        // Compute tree OID — shards get wrapped in a tree (matching git::write_commit)
        let tree_oid = match &self.fractal {
            Fractal::Shard { data, .. } => tree_oid_bytes(&data.encode(), &[] as &[Fractal<E>]),
            Fractal::Fractal { .. } => content_oid(&self.fractal),
        };

        repo.write_tree(&self.fractal);

        let commit_sha = compute_commit_sha(
            &tree_oid,
            self.parent.as_ref().map(|p| p.0 .0.as_str()),
            &author,
            &committer,
            timestamp,
            &self.message.0,
        );

        let sha = Sha(commit_sha);
        let epoch = timestamp.split_whitespace().next().unwrap_or(timestamp);
        let witnessed = Witnessed::new(author, committer, Timestamp(epoch.to_string()));

        let commit = match self.parent {
            None => Commit::full_root(self.fractal, witnessed, self.message, sha.clone()),
            Some(p) => Commit::full_child(self.fractal, witnessed, self.message, p, sha.clone()),
        };

        repo.write_commit(commit.clone());
        commit
    }

    /// Write this draft to a git repository.
    /// Returns a Commit (Root or Child) with SHA and witnessed metadata.
    #[cfg(feature = "git")]
    pub fn write(
        self,
        repo: &git2::Repository,
        committer: Committer,
    ) -> Result<Commit<E>, git2::Error>
    where
        E: Encode,
    {
        let author = self
            .author
            .unwrap_or_else(|| Author::new(&committer.name, &committer.email));
        let oid = crate::git::write_commit(
            repo,
            &self.fractal,
            &author,
            &committer,
            &self.message.0,
            self.parent.as_ref().map(|p| &p.0),
        )?;
        let git_commit = repo.find_commit(oid)?;
        let timestamp = Timestamp(git_commit.time().seconds().to_string());
        let witnessed = Witnessed::new(author, committer, timestamp);
        let sha = Sha(oid.to_string());

        Ok(match self.parent {
            None => Commit::Root {
                fractal: self.fractal,
                witnessed,
                message: self.message,
                sha,
            },
            Some(parent) => Commit::Child {
                fractal: self.fractal,
                witnessed,
                message: self.message,
                parent,
                sha,
            },
        })
    }
}

impl<E> Draftable for Draft<E> {
    type Element = E;

    fn fractal(&self) -> &Fractal<E> {
        &self.fractal
    }

    fn message(&self) -> &Message {
        &self.message
    }

    fn parent(&self) -> Option<&Parent> {
        self.parent.as_ref()
    }
}

impl<E> Commit<E> {
    /// This commit's SHA.
    pub fn sha(&self) -> &Sha {
        match self {
            Commit::Root { sha, .. } => sha,
            Commit::Child { sha, .. } => sha,
        }
    }

    /// Witness metadata: author, committer, timestamp.
    pub fn witnessed(&self) -> &Witnessed {
        match self {
            Commit::Root { witnessed, .. } => witnessed,
            Commit::Child { witnessed, .. } => witnessed,
        }
    }

    /// Create a child draft from this commit.
    pub fn child(&self, message: impl Into<String>, fractal: Fractal<E>) -> Draft<E> {
        Draft {
            fractal,
            message: Message(message.into()),
            parent: Some(Parent(self.sha().clone())),
            author: None,
        }
    }

    /// Construct a Root with full metadata.
    pub(crate) fn full_root(
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        sha: Sha,
    ) -> Self {
        Commit::Root {
            fractal,
            witnessed,
            message,
            sha,
        }
    }

    /// Construct a Child with full metadata.
    pub(crate) fn full_child(
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        parent: Parent,
        sha: Sha,
    ) -> Self {
        Commit::Child {
            fractal,
            witnessed,
            message,
            parent,
            sha,
        }
    }
}

impl<E> Draftable for Commit<E> {
    type Element = E;

    fn fractal(&self) -> &Fractal<E> {
        match self {
            Commit::Root { fractal, .. } => fractal,
            Commit::Child { fractal, .. } => fractal,
        }
    }

    fn message(&self) -> &Message {
        match self {
            Commit::Root { message, .. } => message,
            Commit::Child { message, .. } => message,
        }
    }

    fn parent(&self) -> Option<&Parent> {
        match self {
            Commit::Root { .. } => None,
            Commit::Child { parent, .. } => Some(parent),
        }
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
    use crate::encoding;
    use crate::fragment::Fractal;
    use crate::store::Store;

    fn test_fractal() -> Fractal<String> {
        encoding::encode("hello world")
    }

    fn test_committer() -> Committer {
        Committer::new("Test", "test@test.com")
    }

    const TEST_TIMESTAMP: &str = "1234567890 +0000";

    #[test]
    fn draft_commit_root() {
        let mut store = Store::<String>::new();
        let fractal = test_fractal();
        let draft = Draft::root("initial", fractal);
        let commit = draft.commit(&mut store, test_committer(), TEST_TIMESTAMP);
        assert!(matches!(commit, Commit::Root { .. }));
        assert!(!commit.sha().0.is_empty());
        // Commit should be retrievable from store
        assert!(store.read_commit(commit.sha()).is_some());
    }

    #[test]
    fn draft_commit_child() {
        let mut store = Store::<String>::new();
        let fractal = test_fractal();
        let root = Draft::root("root", fractal.clone()).commit(
            &mut store,
            test_committer(),
            TEST_TIMESTAMP,
        );
        let child =
            root.child("child", fractal)
                .commit(&mut store, test_committer(), "1234567891 +0000");
        assert!(matches!(child, Commit::Child { .. }));
        assert_ne!(child.sha(), root.sha());
    }

    /// Draft::commit() must produce the same SHA as git2 with matching inputs.
    /// Draft::write() uses Signature::now(), so we compare against raw git2 with fixed timestamps.
    #[cfg(feature = "git")]
    #[test]
    fn draft_commit_matches_git() {
        let fractal = test_fractal();
        let author = Author::new("Test", "test@test.com");
        let committer = Committer::new("Test", "test@test.com");
        let timestamp = "1234567890 +0000";
        let epoch: i64 = 1234567890;

        // In-memory via Draft::commit()
        let mut store = Store::<String>::new();
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
