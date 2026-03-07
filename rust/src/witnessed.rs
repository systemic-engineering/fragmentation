#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Author(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committer(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timestamp(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message(pub String);

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

pub fn serialize_witnessed(_w: &Witnessed) -> String {
    todo!("implement serialize_witnessed")
}
