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
    pub fn identity(name: impl Into<String>, email: impl Into<String>) -> Self {
        fn id(f: &Fragment<Blob>) -> Fragment<Blob> {
            f.clone()
        }
        Actor {
            name: name.into(),
            email: email.into(),
            encoder: id,
            decoder: id,
            keys: PlainKeys,
        }
    }
}

impl<A, B, K: Keys> Actor<A, B, K> {
    /// Full constructor with custom encoder, decoder, and keys.
    pub fn new(
        name: impl Into<String>,
        email: impl Into<String>,
        encoder: fn(&Fragment<A>) -> Fragment<B>,
        decoder: fn(&Fragment<B>) -> Fragment<A>,
        keys: K,
    ) -> Self {
        Actor {
            name: name.into(),
            email: email.into(),
            encoder,
            decoder,
            keys,
        }
    }

    /// Actor's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Actor's email.
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Actor's keys.
    pub fn keys(&self) -> &K {
        &self.keys
    }

    /// Encode a fragment from A to B.
    pub fn encode(&self, fragment: &Fragment<A>) -> Fragment<B> {
        (self.encoder)(fragment)
    }

    /// Decode a fragment from B to A.
    pub fn decode(&self, fragment: &Fragment<B>) -> Fragment<A> {
        (self.decoder)(fragment)
    }

    /// Sign an encoded fragment.
    pub fn sign(&self, fragment: Fragment<B>) -> Signed<K, Fragment<B>> {
        self.keys.sign(fragment)
    }

    /// Encrypt an encoded fragment.
    pub fn encrypt(&self, fragment: Fragment<B>) -> Encrypted<K>
    where
        B: Encode,
    {
        self.keys.encrypt(fragment)
    }

    /// Decrypt to an encoded fragment.
    pub fn decrypt(&self, encrypted: &Encrypted<K>) -> Fragment<B>
    where
        B: Decode,
    {
        self.keys.decrypt(encrypted)
    }
}
