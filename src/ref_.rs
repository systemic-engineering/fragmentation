use crate::sha::Sha;

/// A reference: address + label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref {
    pub sha: Sha,
    pub label: String,
}

impl Ref {
    pub fn new(sha: Sha, label: impl Into<String>) -> Self {
        Ref {
            sha,
            label: label.into(),
        }
    }
}
