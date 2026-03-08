use crate::encoding::{Decode, Encode};
use crate::fragment::{Fractal, Fragment};
use crate::keys::{Encrypted, Keys};
use crate::ref_::Ref;

/// Transparent visibility. Content accessible. Key is provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Public<K, T> {
    inner: T,
    key: K,
}

impl<K, T> Public<K, T> {
    pub fn new(inner: T, key: K) -> Self {
        Public { inner, key }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn key(&self) -> &K {
        &self.key
    }
}

impl<K, T: Fragment> Fragment for Public<K, T> {
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
pub struct Protected<K> {
    ref_: Ref,
    ciphertext: Vec<u8>,
    key: K,
}

impl<K> Protected<K> {
    pub fn new(ref_: Ref, ciphertext: Vec<u8>, key: K) -> Self {
        Protected {
            ref_,
            ciphertext,
            key,
        }
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    pub fn key(&self) -> &K {
        &self.key
    }
}

impl<K: Keys> Protected<K> {
    pub fn wrap<E: Encode>(fragment: Fractal<E>, key: K) -> Result<Self, K::Error> {
        let ref_ = fragment.self_ref().clone();
        let encrypted = key.encrypt(fragment)?;
        Ok(Protected {
            ref_,
            ciphertext: encrypted.ciphertext().to_vec(),
            key,
        })
    }

    pub fn unlock<E: Decode>(&self) -> Result<Fractal<E>, K::Error> {
        let encrypted = Encrypted::new(self.ciphertext.clone(), self.key.clone());
        self.key.decrypt(&encrypted)
    }
}

impl<K> Fragment for Protected<K> {
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
pub struct Private<K> {
    ref_: Ref,
    key: K,
}

impl<K> Private<K> {
    pub fn new(ref_: Ref, key: K) -> Self {
        Private { ref_, key }
    }

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn seal<T: Fragment>(fragment: &T, key: K) -> Self {
        Private {
            ref_: fragment.self_ref().clone(),
            key,
        }
    }
}

impl<K> Fragment for Private<K> {
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
