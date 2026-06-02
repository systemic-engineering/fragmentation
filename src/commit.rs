use crate::encoding::Encode;
use crate::fragment::{content_oid, tree_oid_bytes, ContentAddressed, Fragmentable, TreeShaped};
use crate::repo::Repo;
use crate::sha::HashAlg;
use crate::spectral_coordinate::SpectralCoordinate;
use crate::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

/// Typed reference to a parent commit. Not a raw SHA — a graph edge.
///
/// Default hash is `SpectralCoordinate<5>` — the substrate hash per
/// `docs/specs/mirror-native-vcs.md` §4.6. The git adapter overrides to
/// `Sha` at its boundary per §4.7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parent<H: HashAlg = SpectralCoordinate<5>>(pub H);

/// The commit graph interface. Draft and Commit both implement this.
/// A signed commit (Public<K, T: Draftable>) also implements it.
pub trait Draftable {
    type Node;
    type Hash: HashAlg;
    fn node(&self) -> &Self::Node;
    fn message(&self) -> &Message;
    fn parent(&self) -> Option<&Parent<Self::Hash>>;
}

/// A commit before it has been written to git.
/// Has content and intent, but no SHA and no witnessed metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft<N, H: HashAlg = SpectralCoordinate<5>> {
    node: N,
    message: Message,
    parent: Option<Parent<H>>,
    author: Option<Author>,
}

/// A commit that has been written to git.
/// Root has no parent. Child has a parent. The enum discriminant carries the distinction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Commit<N, H: HashAlg = SpectralCoordinate<5>> {
    /// Terminal in the commit graph. No parent.
    Root {
        node: N,
        witnessed: Witnessed,
        message: Message,
        sha: H,
    },
    /// Has a parent. Always.
    Child {
        node: N,
        witnessed: Witnessed,
        message: Message,
        parent: Parent<H>,
        sha: H,
    },
}

impl<N, H: HashAlg> Draft<N, H> {
    /// Create a root draft (no parent).
    pub fn root(message: impl Into<String>, node: N) -> Self {
        Draft {
            node,
            message: Message(message.into()),
            parent: None,
            author: None,
        }
    }

    /// Create a draft with a parent.
    pub fn new(message: impl Into<String>, node: N, parent: Parent<H>) -> Self {
        Draft {
            node,
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

    /// Consume the draft and return its parts.
    /// Used by git integration crates that need access to internal fields.
    pub fn into_parts(self) -> (N, Message, Option<Parent<H>>) {
        (self.node, self.message, self.parent)
    }

    /// Commit this draft to a repo. Computes git-compatible commit SHA.
    ///
    /// `timestamp` is in git format: "{epoch} {tz_offset}", e.g. "1234567890 +0000".
    pub fn commit(
        self,
        repo: &mut impl Repo<Node = N, Hash = H>,
        committer: Committer,
        timestamp: &str,
    ) -> Commit<N, H>
    where
        N: Fragmentable<Hash = H> + Clone,
    {
        let author = self
            .author
            .unwrap_or_else(|| Author::new(&committer.name, &committer.email));

        // Compute tree OID — shards get wrapped in a tree (matching git::write_commit)
        let tree_oid = if self.node.is_shard() {
            tree_oid_bytes(&self.node.data().encode(), self.node.children())
        } else {
            content_oid(&self.node)
        };

        repo.write_tree(&self.node);

        let commit_sha = compute_commit_sha(
            &tree_oid,
            self.parent.as_ref().map(|p| p.0.as_str()),
            &author,
            &committer,
            timestamp,
            &self.message.0,
        );

        let sha = H::from_hex(commit_sha);
        let epoch = timestamp.split_whitespace().next().unwrap_or(timestamp);
        let witnessed = Witnessed::new(author, committer, Timestamp(epoch.to_string()));

        let commit = match self.parent {
            None => Commit::full_root(self.node, witnessed, self.message, sha.clone()),
            Some(p) => Commit::full_child(self.node, witnessed, self.message, p, sha.clone()),
        };

        repo.write_commit(commit.clone());
        commit
    }
}

impl<N, H: HashAlg> Draftable for Draft<N, H> {
    type Node = N;
    type Hash = H;

    fn node(&self) -> &N {
        &self.node
    }

    fn message(&self) -> &Message {
        &self.message
    }

    fn parent(&self) -> Option<&Parent<H>> {
        self.parent.as_ref()
    }
}

impl<N, H: HashAlg> Commit<N, H> {
    /// This commit's hash.
    pub fn sha(&self) -> &H {
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
    pub fn child(&self, message: impl Into<String>, node: N) -> Draft<N, H> {
        Draft {
            node,
            message: Message(message.into()),
            parent: Some(Parent(self.sha().clone())),
            author: None,
        }
    }

    /// Construct a Root with full metadata.
    pub fn full_root(node: N, witnessed: Witnessed, message: Message, sha: H) -> Self {
        Commit::Root {
            node,
            witnessed,
            message,
            sha,
        }
    }

    /// Construct a Child with full metadata.
    pub fn full_child(
        node: N,
        witnessed: Witnessed,
        message: Message,
        parent: Parent<H>,
        sha: H,
    ) -> Self {
        Commit::Child {
            node,
            witnessed,
            message,
            parent,
            sha,
        }
    }
}

impl<N, H: HashAlg> Draftable for Commit<N, H> {
    type Node = N;
    type Hash = H;

    fn node(&self) -> &N {
        match self {
            Commit::Root { node, .. } => node,
            Commit::Child { node, .. } => node,
        }
    }

    fn message(&self) -> &Message {
        match self {
            Commit::Root { message, .. } => message,
            Commit::Child { message, .. } => message,
        }
    }

    fn parent(&self) -> Option<&Parent<H>> {
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
    use crate::sha::Sha;
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
        let mut store = Store::<Fractal<String>>::new();
        let fractal = test_fractal();
        let draft: Draft<Fractal<String>, Sha> = Draft::root("initial", fractal);
        let commit = draft.commit(&mut store, test_committer(), TEST_TIMESTAMP);
        assert!(matches!(commit, Commit::Root { .. }));
        assert!(!commit.sha().0.is_empty());
        // Commit should be retrievable from store
        assert!(store.read_commit(commit.sha()).is_some());
    }

    #[test]
    fn draft_commit_child() {
        let mut store = Store::<Fractal<String>>::new();
        let fractal = test_fractal();
        let root = Draft::<Fractal<String>, Sha>::root("root", fractal.clone()).commit(
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
}
