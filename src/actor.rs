use crate::encoding::{Decode, Encode};
use crate::fragment::{Blob, Fragment};
use crate::keys::{Encrypted, Keys, PlainKeys, Signed};

/// Witness identity with encoding boundary.
///
/// An actor doesn't tag the encoding — it does the encoding.
/// `encoder` transforms `Fragment<A>` to `Fragment<B>`,
/// `decoder` reverses. Keys handle visibility layers.
///
/// fn pointers, not closures — simple, cloneable, deterministic.
#[derive(Clone)]
pub struct Actor<A = Blob, B = Blob, K: Keys = PlainKeys> {
    name: String,
    email: String,
    encoder: fn(&Fragment<A>) -> Fragment<B>,
    decoder: fn(&Fragment<B>) -> Fragment<A>,
    keys: K,
}

impl Actor {
    /// Default actor: bytes-to-bytes identity, plain keys.
    pub fn identity(_name: impl Into<String>, _email: impl Into<String>) -> Self {
        todo!()
    }
}

impl<A, B, K: Keys> Actor<A, B, K> {
    /// Full constructor with custom encoder, decoder, and keys.
    pub fn new(
        _name: impl Into<String>,
        _email: impl Into<String>,
        _encoder: fn(&Fragment<A>) -> Fragment<B>,
        _decoder: fn(&Fragment<B>) -> Fragment<A>,
        _keys: K,
    ) -> Self {
        todo!()
    }

    /// Actor's name.
    pub fn name(&self) -> &str {
        todo!()
    }

    /// Actor's email.
    pub fn email(&self) -> &str {
        todo!()
    }

    /// Actor's keys.
    pub fn keys(&self) -> &K {
        todo!()
    }

    /// Encode a fragment from A to B.
    pub fn encode(&self, _fragment: &Fragment<A>) -> Fragment<B> {
        todo!()
    }

    /// Decode a fragment from B to A.
    pub fn decode(&self, _fragment: &Fragment<B>) -> Fragment<A> {
        todo!()
    }

    /// Sign an encoded fragment.
    pub fn sign(&self, _fragment: Fragment<B>) -> Signed<K, Fragment<B>> {
        todo!()
    }

    /// Encrypt an encoded fragment.
    pub fn encrypt(&self, _fragment: Fragment<B>) -> Encrypted<K>
    where
        B: Encode,
    {
        todo!()
    }

    /// Decrypt to an encoded fragment.
    pub fn decrypt(&self, _encrypted: &Encrypted<K>) -> Fragment<B>
    where
        B: Decode,
    {
        todo!()
    }
}
