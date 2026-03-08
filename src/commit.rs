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
    pub fn new(message: impl Into<String>, fractal: Fractal<E>, parent: Sha) -> Self {
        Commit {
            fractal,
            witnessed: Witnessed::empty(),
            message: Message(message.into()),
            parent: Some(parent),
            sha: None,
        }
    }

    /// Create a root commit (no parent).
    pub fn root(message: impl Into<String>, fractal: Fractal<E>) -> Self {
        Commit {
            fractal,
            witnessed: Witnessed::empty(),
            message: Message(message.into()),
            parent: None,
            sha: None,
        }
    }

    /// Stamp the Author.
    pub fn authored(mut self, author: Author) -> Self {
        self.witnessed.author = author;
        self
    }

    /// Create a child commit. Requires this commit to have been written (has SHA).
    pub fn child(&self, message: impl Into<String>, fractal: Fractal<E>) -> Commit<E> {
        let sha = self
            .sha
            .as_ref()
            .expect("cannot create child of unwritten commit");
        Commit {
            fractal,
            witnessed: Witnessed::empty(),
            message: Message(message.into()),
            parent: Some(sha.clone()),
            sha: None,
        }
    }

    /// Write this commit to a git repository.
    /// Stamps committer and timestamp, returns Self with SHA populated.
    #[cfg(feature = "git")]
    pub fn write(
        mut self,
        repo: &git2::Repository,
        committer: Committer,
    ) -> Result<Self, git2::Error>
    where
        E: crate::encoding::Encode,
    {
        self.witnessed.committer = committer;
        if self.witnessed.author.name.is_empty() {
            self.witnessed.author = Author::new(
                &self.witnessed.committer.name,
                &self.witnessed.committer.email,
            );
        }
        let oid = crate::git::write_commit(repo, &self)?;
        let git_commit = repo.find_commit(oid)?;
        self.witnessed.timestamp = Timestamp(git_commit.time().seconds().to_string());
        self.sha = Some(Sha(oid.to_string()));
        Ok(self)
    }

    /// The fractal tree this commit captures.
    pub fn fractal(&self) -> &Fractal<E> {
        &self.fractal
    }

    /// Witness metadata: author, committer, timestamp.
    pub fn witnessed(&self) -> &Witnessed {
        &self.witnessed
    }

    /// The commit message.
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Parent commit SHA, if any.
    pub fn parent(&self) -> Option<&Sha> {
        self.parent.as_ref()
    }

    /// This commit's SHA, if written.
    pub fn sha(&self) -> Option<&Sha> {
        self.sha.as_ref()
    }

    /// Construct with full metadata (used by read_commit).
    pub(crate) fn full(
        fractal: Fractal<E>,
        witnessed: Witnessed,
        message: Message,
        parent: Option<Sha>,
        sha: Sha,
    ) -> Self {
        Commit {
            fractal,
            witnessed,
            message,
            parent,
            sha: Some(sha),
        }
    }
}
