use crate::sha::Sha;

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

pub fn serialize_ref(_r: &Ref) -> String {
    todo!("implement serialize_ref")
}
