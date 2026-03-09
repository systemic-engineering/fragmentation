use crate::encoding::{Decode, Encode};
use crate::fragment::{Blob, Fractal};
use crate::keys::{Encrypted, Keys, Local};
use crate::visibility::Public;

/// Witness identity with encoding boundary.
///
/// An actor doesn't tag the encoding — it does the encoding.
/// `encoder` transforms `Fractal<A>` to `Fractal<B>`,
/// `decoder` reverses. Keys handle visibility layers.
///
/// fn pointers, not closures — simple, cloneable, deterministic.
#[derive(Clone)]
pub struct Actor<A = Blob, B = Blob, K: Keys = Local> {
    name: String,
    email: String,
    encoder: fn(&Fractal<A>) -> Fractal<B>,
    decoder: fn(&Fractal<B>) -> Fractal<A>,
    keys: K,
}

impl Actor {
    /// Default actor: bytes-to-bytes identity, local keys (plain).
    pub fn identity(name: impl Into<String>, email: impl Into<String>) -> Self {
        fn id(f: &Fractal<Blob>) -> Fractal<Blob> {
            f.clone()
        }
        Actor {
            name: name.into(),
            email: email.into(),
            encoder: id,
            decoder: id,
            keys: Local::None,
        }
    }
}

impl<A, B, K: Keys> Actor<A, B, K> {
    /// Full constructor with custom encoder, decoder, and keys.
    pub fn new(
        name: impl Into<String>,
        email: impl Into<String>,
        encoder: fn(&Fractal<A>) -> Fractal<B>,
        decoder: fn(&Fractal<B>) -> Fractal<A>,
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
    pub fn encode(&self, fragment: &Fractal<A>) -> Fractal<B> {
        (self.encoder)(fragment)
    }

    /// Decode a fragment from B to A.
    pub fn decode(&self, fragment: &Fractal<B>) -> Fractal<A> {
        (self.decoder)(fragment)
    }

    /// Sign an encoded fragment, wrapping it in Public visibility.
    pub fn sign(&self, fragment: Fractal<B>) -> Result<Public<K, Fractal<B>>, K::Error> {
        let sig = self.keys.sign(&fragment)?;
        Ok(Public::new(fragment, sig))
    }

    /// Encrypt an encoded fragment.
    pub fn encrypt(&self, fragment: Fractal<B>) -> Result<Encrypted<K>, K::Error>
    where
        B: Encode,
    {
        self.keys.encrypt(fragment)
    }

    /// Decrypt to an encoded fragment.
    pub fn decrypt(&self, encrypted: &Encrypted<K>) -> Result<Fractal<B>, K::Error>
    where
        B: Decode,
    {
        self.keys.decrypt(encrypted)
    }

    /// Write a draft to a git repository as this actor.
    /// If no author is set on the draft, actor becomes both author and committer.
    /// If authored, actor becomes committer only.
    #[cfg(feature = "git")]
    pub fn commit<E: crate::encoding::Encode>(
        &self,
        draft: crate::commit::Draft<E>,
        repo: &git2::Repository,
    ) -> Result<crate::commit::Commit<E>, git2::Error> {
        use crate::witnessed::{Author, Committer};
        let d = if draft.author().is_none() {
            draft.authored(Author::new(&self.name, &self.email))
        } else {
            draft
        };
        d.write(repo, Committer::new(&self.name, &self.email))
    }
}
