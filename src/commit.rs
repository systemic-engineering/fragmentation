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
}

impl<E> Commit<E> {
    /// Create a commit with a parent.
    pub fn new(fractal: Fractal<E>, message: impl Into<String>, parent: Sha) -> Self {
        todo!()
    }

    /// Create a root commit (no parent).
    pub fn root(fractal: Fractal<E>, message: impl Into<String>) -> Self {
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

    /// Stamp the Author and timestamp.
    pub(crate) fn with_author(self, author: Author, timestamp: Timestamp) -> Self {
        todo!()
    }

    /// Stamp the Committer.
    pub(crate) fn with_committer(self, committer: Committer) -> Self {
        todo!()
    }

    /// Construct with full metadata (used by read_commit).
    pub(crate) fn full(
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        parent: Option<Sha>,
    ) -> Self {
        todo!()
    }
}
