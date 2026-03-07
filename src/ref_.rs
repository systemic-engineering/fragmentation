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

/// Deterministic canonical serialization of a ref.
pub fn serialize_ref(r: &Ref) -> String {
    format!("ref:{}:{}", r.sha.0, r.label)
}
