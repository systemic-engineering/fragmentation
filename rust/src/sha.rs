use sha2::{Digest, Sha256};

/// Content-addressed hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sha(pub String);

/// Raw SHA-256 hash of a string.
pub fn hash(data: &str) -> Sha {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    Sha(hex::encode(result))
}
