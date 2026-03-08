use crate::fragment::Fractal;
use crate::sha::Sha;
use crate::witnessed::{Author, Committer, Message, Witnessed};

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
        todo!()
    }

    /// Create a draft with a parent.
    pub fn new(message: impl Into<String>, fractal: Fractal<E>, parent: Parent) -> Self {
        todo!()
    }

    /// Stamp the Author.
    pub fn authored(mut self, author: Author) -> Self {
        todo!()
    }

    /// The author, if set.
    pub fn author(&self) -> Option<&Author> {
        todo!()
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
        E: crate::encoding::Encode,
    {
        todo!()
    }
}

impl<E> Draftable for Draft<E> {
    type Element = E;

    fn fractal(&self) -> &Fractal<E> {
        todo!()
    }

    fn message(&self) -> &Message {
        todo!()
    }

    fn parent(&self) -> Option<&Parent> {
        todo!()
    }
}

impl<E> Commit<E> {
    /// This commit's SHA.
    pub fn sha(&self) -> &Sha {
        todo!()
    }

    /// Witness metadata: author, committer, timestamp.
    pub fn witnessed(&self) -> &Witnessed {
        todo!()
    }

    /// Create a child draft from this commit.
    pub fn child(&self, message: impl Into<String>, fractal: Fractal<E>) -> Draft<E> {
        todo!()
    }

    /// Construct a Root with full metadata (used by read_commit).
    pub(crate) fn full_root(
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        sha: Sha,
    ) -> Self {
        todo!()
    }

    /// Construct a Child with full metadata (used by read_commit).
    pub(crate) fn full_child(
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        parent: Parent,
        sha: Sha,
    ) -> Self {
        todo!()
    }
}

impl<E> Draftable for Commit<E> {
    type Element = E;

    fn fractal(&self) -> &Fractal<E> {
        todo!()
    }

    fn message(&self) -> &Message {
        todo!()
    }

    fn parent(&self) -> Option<&Parent> {
        todo!()
    }
}
