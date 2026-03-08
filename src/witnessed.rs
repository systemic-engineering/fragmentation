/// Who wrote the content. Who made the decision. Who holds the intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Author {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Author {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Who ran the process. Who executed. Who was the mechanism.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committer {
    pub name: String,
    pub email: String,
}

impl Committer {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Committer {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// When the observation happened. Opaque string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timestamp(pub String);

/// The commit message. What happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message(pub String);

/// Git commit metadata. Who was here when this happened.
/// Message lives on Commit, not here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witnessed {
    pub author: Author,
    pub committer: Committer,
    pub timestamp: Timestamp,
}

impl Witnessed {
    pub fn new(author: Author, committer: Committer, timestamp: Timestamp) -> Self {
        todo!()
    }

    /// Empty witness — to be filled by actor.author() and actor.commit().
    pub fn empty() -> Self {
        todo!()
    }
}
