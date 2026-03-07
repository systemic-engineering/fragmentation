/// Who wrote the content. Who made the decision. Who holds the intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Author(pub String);

/// Who ran the process. Who executed. Who was the mechanism.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committer(pub String);

/// When the observation happened. Opaque string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timestamp(pub String);

/// The witness's account of what happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message(pub String);

/// Git commit metadata. Who was here when this happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witnessed {
    pub author: Author,
    pub committer: Committer,
    pub timestamp: Timestamp,
    pub message: Message,
}

impl Witnessed {
    pub fn new(
        author: Author,
        committer: Committer,
        timestamp: Timestamp,
        message: Message,
    ) -> Self {
        Witnessed {
            author,
            committer,
            timestamp,
            message,
        }
    }
}
