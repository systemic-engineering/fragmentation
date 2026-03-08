use crate::encoding::{Decode, Encode};
use crate::fragment::{Fractal, Fragment};
use crate::keys::Keys;
use crate::ref_::Ref;

/// Transparent visibility. Content accessible. Key is provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Public<K, T> {
    inner: T,
    key: K,
}

impl<K, T> Public<K, T> {
    pub fn new(_inner: T, _key: K) -> Self {
        todo!()
    }

    pub fn inner(&self) -> &T {
        todo!()
    }

    pub fn into_inner(self) -> T {
        todo!()
    }

    pub fn key(&self) -> &K {
        todo!()
    }
}

impl<K, T: Fragment> Fragment for Public<K, T> {
    type Data = T::Data;

    fn self_ref(&self) -> &Ref {
        todo!()
    }

    fn data(&self) -> &T::Data {
        todo!()
    }

    fn children(&self) -> &[Self] {
        todo!()
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
    pub fn new(_ref_: Ref, _ciphertext: Vec<u8>, _key: K) -> Self {
        todo!()
    }

    pub fn ciphertext(&self) -> &[u8] {
        todo!()
    }

    pub fn key(&self) -> &K {
        todo!()
    }
}

impl<K: Keys> Protected<K> {
    pub fn wrap<E: Encode>(_fragment: Fractal<E>, _key: K) -> Result<Self, K::Error> {
        todo!()
    }

    pub fn unlock<E: Decode>(&self) -> Result<Fractal<E>, K::Error> {
        todo!()
    }
}

impl<K> Fragment for Protected<K> {
    type Data = Vec<u8>;

    fn self_ref(&self) -> &Ref {
        todo!()
    }

    fn data(&self) -> &Vec<u8> {
        todo!()
    }

    fn children(&self) -> &[Self] {
        todo!()
    }
}

/// Proof of existence only. No content travels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Private<K> {
    ref_: Ref,
    key: K,
}

impl<K> Private<K> {
    pub fn new(_ref_: Ref, _key: K) -> Self {
        todo!()
    }

    pub fn key(&self) -> &K {
        todo!()
    }

    pub fn seal<T: Fragment>(_fragment: &T, _key: K) -> Self {
        todo!()
    }
}

impl<K> Fragment for Private<K> {
    type Data = ();

    fn self_ref(&self) -> &Ref {
        todo!()
    }

    fn data(&self) -> &() {
        todo!()
    }

    fn children(&self) -> &[Self] {
        todo!()
    }
}
