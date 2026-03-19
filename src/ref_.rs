use crate::sha::Sha;

/// A reference: address + label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref<H = Sha> {
    pub sha: H,
    pub label: String,
}

impl<H> Ref<H> {
    pub fn new(sha: H, label: impl Into<String>) -> Self {
        Ref {
            sha,
            label: label.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha::HashAlg;

    #[test]
    fn ref_generic_with_sha_default() {
        // Ref without type parameter uses Sha
        let r: Ref = Ref::new(Sha("abc".into()), "label");
        assert_eq!(r.sha.as_str(), "abc");
        assert_eq!(r.label, "label");
    }

    #[test]
    fn ref_generic_with_explicit_sha() {
        let r: Ref<Sha> = Ref::new(Sha("def".into()), "test");
        assert_eq!(r.sha.as_str(), "def");
    }
}
