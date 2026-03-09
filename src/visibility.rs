use crate::encoding::{Decode, Encode};
use crate::fragment::{Fractal, Fragmentable};
use crate::keys::{Encrypted, Keys, Signature};
use crate::ref_::Ref;

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

    fn self_ref(&self) -> &Ref {
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
pub struct Protected<K: Keys> {
    ref_: Ref,
    ciphertext: Vec<u8>,
    signature: Signature<K>,
}

impl<K: Keys> Protected<K> {
    pub fn new(ref_: Ref, ciphertext: Vec<u8>, signature: Signature<K>) -> Self {
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

impl<K: Keys> Protected<K> {
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

impl<K: Keys> Fragmentable for Protected<K> {
    type Data = Vec<u8>;

    fn self_ref(&self) -> &Ref {
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
pub struct Private<K: Keys> {
    ref_: Ref,
    signature: Signature<K>,
}

impl<K: Keys> Private<K> {
    pub fn new(ref_: Ref, signature: Signature<K>) -> Self {
        Private { ref_, signature }
    }

    pub fn signature(&self) -> &Signature<K> {
        &self.signature
    }

    pub fn key(&self) -> &K {
        self.signature.key()
    }

    pub fn seal<T: Fragmentable>(fragment: &T, signature: Signature<K>) -> Self {
        Private {
            ref_: fragment.self_ref().clone(),
            signature,
        }
    }
}

impl<K: Keys, T: crate::commit::Draftable> crate::commit::Draftable for Public<K, T> {
    type Element = T::Element;

    fn fractal(&self) -> &Fractal<Self::Element> {
        self.inner.fractal()
    }

    fn message(&self) -> &crate::witnessed::Message {
        self.inner.message()
    }

    fn parent(&self) -> Option<&crate::commit::Parent> {
        self.inner.parent()
    }
}

impl<K: Keys> Fragmentable for Private<K> {
    type Data = ();

    fn self_ref(&self) -> &Ref {
        &self.ref_
    }

    fn data(&self) -> &() {
        &()
    }

    fn children(&self) -> &[Self] {
        &[]
    }
}
