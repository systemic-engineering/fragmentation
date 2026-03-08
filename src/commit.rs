use crate::fragment::Fractal;
use crate::sha::Sha;
use crate::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

/// The atomic unit. A fractal committed with witness metadata.
///
/// Two potentially different actors: Alice writes the patch (Author),
/// Bob applies it (Committer). The Message is on the Commit, not the
/// Witnessed — it's what happened, not who was there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit<E> {
    fractal: Fractal<E>,
    witnessed: Witnessed,
    message: Message,
    parent: Option<Sha>,
    sha: Option<Sha>,
}

impl<E> Commit<E> {
    /// Create a commit with a parent.
    pub fn new(_message: impl Into<String>, _fractal: Fractal<E>, _parent: Sha) -> Self {
        todo!()
    }

    /// Create a root commit (no parent).
    pub fn root(_message: impl Into<String>, _fractal: Fractal<E>) -> Self {
        todo!()
    }

    /// Stamp the Author.
    pub fn authored(self, _author: Author) -> Self {
        todo!()
    }

    /// Create a child commit. Requires this commit to have been written (has SHA).
    pub fn child(&self, _message: impl Into<String>, _fractal: Fractal<E>) -> Commit<E> {
        todo!()
    }

    /// Write this commit to a git repository.
    /// Stamps committer and timestamp, returns Self with SHA populated.
    #[cfg(feature = "git")]
    pub fn write(
        self,
        _repo: &git2::Repository,
        _committer: Committer,
    ) -> Result<Self, git2::Error>
    where
        E: crate::encoding::Encode,
    {
        todo!()
    }

    /// The fractal tree this commit captures.
    pub fn fractal(&self) -> &Fractal<E> {
        todo!()
    }

    /// Witness metadata: author, committer, timestamp.
    pub fn witnessed(&self) -> &Witnessed {
        todo!()
    }

    /// The commit message.
    pub fn message(&self) -> &Message {
        todo!()
    }

    /// Parent commit SHA, if any.
    pub fn parent(&self) -> Option<&Sha> {
        todo!()
    }

    /// This commit's SHA, if written.
    pub fn sha(&self) -> Option<&Sha> {
        todo!()
    }

    /// Construct with full metadata (used by read_commit).
    pub(crate) fn full(
        _fractal: Fractal<E>,
        _witnessed: Witnessed,
        _message: Message,
        _parent: Option<Sha>,
        _sha: Sha,
    ) -> Self {
        todo!()
    }
}
