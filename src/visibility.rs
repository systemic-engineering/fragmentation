use crate::encoding::{Decode, Encode};
use crate::fragment::{Fractal, Fragmentable};
use crate::keys::{Encrypted, Keys, Signature};
use crate::ref_::Ref;
use crate::sha::{HashAlg, Sha};

/// Visible, attributed, proven content. Signature carries both key and proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Public<K: Keys, T> {
    inner: T,
    signature: Signature<K>,
}

impl<K: Keys, T> Public<K, T> {
    pub fn new(inner: T, signature: Signature<K>) -> Self {
        Public { inner, signature }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn signature(&self) -> &Signature<K> {
        &self.signature
    }

    pub fn key(&self) -> &K {
        self.signature.key()
    }
}

impl<K: Keys, T: Fragmentable> Fragmentable for Public<K, T> {
    type Data = T::Data;
    type Hash = T::Hash;

    fn self_ref(&self) -> &Ref<T::Hash> {
        self.inner.self_ref()
    }

    fn data(&self) -> &T::Data {
        self.inner.data()
    }

    fn children(&self) -> &[Self] {
        &[]
    }
}

/// Encrypted visibility. Content accessible with key. Ref is plaintext address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Protected<K: Keys, H: HashAlg = Sha> {
    ref_: Ref<H>,
    ciphertext: Vec<u8>,
    signature: Signature<K>,
}

impl<K: Keys, H: HashAlg> Protected<K, H> {
    pub fn new(ref_: Ref<H>, ciphertext: Vec<u8>, signature: Signature<K>) -> Self {
        Protected {
            ref_,
            ciphertext,
            signature,
        }
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    pub fn signature(&self) -> &Signature<K> {
        &self.signature
    }

    pub fn key(&self) -> &K {
        self.signature.key()
    }
}

/// Encryption/decryption requires `Keys` which currently operates on `Fractal<E, Sha>`.
/// When `Keys` becomes hash-generic, these methods can move to the `H: HashAlg` impl block.
impl<K: Keys> Protected<K, Sha> {
    pub fn wrap<E: Encode>(
        fragment: Fractal<E>,
        signature: Signature<K>,
    ) -> Result<Self, K::Error> {
        let ref_ = fragment.self_ref().clone();
        let encrypted = signature.key().encrypt(fragment)?;
        Ok(Protected {
            ref_,
            ciphertext: encrypted.ciphertext().to_vec(),
            signature,
        })
    }

    pub fn unlock<E: Decode>(&self) -> Result<Fractal<E>, K::Error> {
        let encrypted = Encrypted::new(self.ciphertext.clone(), self.signature.key().clone());
        self.signature.key().decrypt(&encrypted)
    }
}

impl<K: Keys, H: HashAlg> Fragmentable for Protected<K, H> {
    type Data = Vec<u8>;
    type Hash = H;

    fn self_ref(&self) -> &Ref<H> {
        &self.ref_
    }

    fn data(&self) -> &Vec<u8> {
        &self.ciphertext
    }

    fn children(&self) -> &[Self] {
        &[]
    }
}

/// Proof of existence only. No content travels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Private<K: Keys, H: HashAlg = Sha> {
    ref_: Ref<H>,
    signature: Signature<K>,
}

impl<K: Keys, H: HashAlg> Private<K, H> {
    pub fn new(ref_: Ref<H>, signature: Signature<K>) -> Self {
        Private { ref_, signature }
    }

    pub fn signature(&self) -> &Signature<K> {
        &self.signature
    }

    pub fn key(&self) -> &K {
        self.signature.key()
    }

    pub fn seal<T: Fragmentable<Hash = H>>(fragment: &T, signature: Signature<K>) -> Self {
        Private {
            ref_: fragment.self_ref().clone(),
            signature,
        }
    }
}

impl<K: Keys, T: crate::commit::Draftable> crate::commit::Draftable for Public<K, T> {
    type Node = T::Node;
    type Hash = T::Hash;

    fn node(&self) -> &Self::Node {
        self.inner.node()
    }

    fn message(&self) -> &crate::witnessed::Message {
        self.inner.message()
    }

    fn parent(&self) -> Option<&crate::commit::Parent<Self::Hash>> {
        self.inner.parent()
    }
}

impl<K: Keys, H: HashAlg> Fragmentable for Private<K, H> {
    type Data = ();
    type Hash = H;

    fn self_ref(&self) -> &Ref<H> {
        &self.ref_
    }

    fn data(&self) -> &() {
        &()
    }

    fn children(&self) -> &[Self] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fragment::{self, Fractal, Fragmentable};
    use crate::keys::PlainKeys;
    use crate::ref_::Ref;
    use crate::sha::{HashAlg, Sha};

    // -- helpers --

    fn make_shard(label: &str) -> Fractal<String> {
        let r = Ref::new(Sha(fragment::blob_oid(label)), label);
        Fractal::shard(r, label)
    }

    // -- Public: hash propagation --

    #[test]
    fn public_fragmentable_propagates_hash_type() {
        let shard = make_shard("test");
        let sig = PlainKeys.sign(&shard).unwrap();
        let public = Public::new(shard.clone(), sig);
        // Hash type propagated from inner T
        assert_eq!(
            Fragmentable::self_ref(&public).sha.as_str(),
            shard.self_ref().sha.as_str()
        );
        assert_eq!(Fragmentable::data(&public), "test");
    }

    #[test]
    fn public_with_oid_hash_type() {
        // Construct a Fractal<String, prism_core::Oid> manually
        let oid = prism_core::Oid::hash(b"hello");
        let r = Ref::new(oid, "hello");
        let shard: Fractal<String, prism_core::Oid> = Fractal::shard(r, "hello");

        // PlainKeys.sign only works with Fractal<E, Sha>, so we construct
        // the Public directly with a plain signature
        let sig = Signature::new(PlainKeys, vec![]);
        let public = Public::new(shard.clone(), sig);

        // The Fragmentable impl should propagate prism_core::Oid
        let ref_ = Fragmentable::self_ref(&public);
        assert_eq!(ref_.sha.as_str(), shard.self_ref().sha.as_str());
    }

    // -- Protected: generic struct --

    #[test]
    fn protected_with_default_sha() {
        let shard = make_shard("secret");
        let sig = PlainKeys.sign(&shard).unwrap();
        let protected = Protected::<PlainKeys>::wrap(shard, sig).unwrap();
        assert!(!protected.ciphertext().is_empty());
    }

    #[test]
    fn protected_with_oid_hash() {
        let oid = prism_core::Oid::hash(b"protected-data");
        let r = Ref::new(oid, "protected");
        let protected: Protected<PlainKeys, prism_core::Oid> = Protected::new(
            r.clone(),
            b"ciphertext".to_vec(),
            Signature::new(PlainKeys, vec![]),
        );
        // Fragmentable should use Oid as Hash
        assert_eq!(
            Fragmentable::self_ref(&protected).sha.as_str(),
            r.sha.as_str()
        );
        assert_eq!(Fragmentable::data(&protected), &b"ciphertext".to_vec());
    }

    // -- Private: generic struct --

    #[test]
    fn private_with_default_sha() {
        let shard = make_shard("sealed");
        let sig = PlainKeys.sign(&shard).unwrap();
        let private = Private::<PlainKeys>::seal(&shard, sig);
        assert_eq!(
            Fragmentable::self_ref(&private).sha.as_str(),
            shard.self_ref().sha.as_str()
        );
    }

    #[test]
    fn private_with_oid_hash() {
        let oid = prism_core::Oid::hash(b"private-data");
        let r = Ref::new(oid.clone(), "private");
        let shard: Fractal<String, prism_core::Oid> = Fractal::shard(r, "private");
        let sig = Signature::new(PlainKeys, vec![]);
        let private: Private<PlainKeys, prism_core::Oid> = Private::seal(&shard, sig);
        // Hash type = Oid, content address matches
        assert_eq!(Fragmentable::self_ref(&private).sha.as_str(), oid.as_str());
    }

    #[test]
    fn private_new_with_oid() {
        let oid = prism_core::Oid::hash(b"direct-construct");
        let r = Ref::new(oid.clone(), "direct");
        let sig = Signature::new(PlainKeys, vec![]);
        let private: Private<PlainKeys, prism_core::Oid> = Private::new(r, sig);
        assert_eq!(Fragmentable::self_ref(&private).sha.as_str(), oid.as_str());
        assert_eq!(Fragmentable::data(&private), &());
    }

    // -- Draftable propagation --

    #[test]
    fn public_draftable_propagates_hash() {
        use crate::commit::{Draft, Draftable};
        let shard = make_shard("draftable");
        let draft = Draft::<Fractal<String>>::root("test", shard.clone());
        let sig = Signature::new(PlainKeys, vec![]);
        let public = Public::new(draft, sig);
        assert_eq!(public.message().0, "test");
        assert!(public.parent().is_none());
    }
}
