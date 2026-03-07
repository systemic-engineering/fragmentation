use crate::encoding::{Decode, Encode};
use crate::fragment::Fragment;

/// Signed content: the inner value, the proof, and who signed it.
/// No PhantomData. Every field is real.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signed<K, T> {
    inner: T,
    signature: Vec<u8>,
    signer: K,
}

impl<K, T> Signed<K, T> {
    /// Construct a signed wrapper.
    pub fn new(inner: T, signature: Vec<u8>, signer: K) -> Self {
        Signed {
            inner,
            signature,
            signer,
        }
    }

    /// Access the signed content.
    pub fn inner(&self) -> &T {
        todo!()
    }

    /// Access the signature bytes.
    pub fn signature(&self) -> &[u8] {
        todo!()
    }

    /// Access the signer.
    pub fn signer(&self) -> &K {
        todo!()
    }

    /// Consume the wrapper, returning the inner value.
    pub fn into_inner(self) -> T {
        todo!()
    }
}

/// Encrypted content: opaque bytes and who it's encrypted for.
/// No type parameter for the content — it's opaque until decrypted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encrypted<K> {
    ciphertext: Vec<u8>,
    key: K,
}

impl<K> Encrypted<K> {
    /// Construct an encrypted wrapper.
    pub fn new(ciphertext: Vec<u8>, key: K) -> Self {
        Encrypted { ciphertext, key }
    }

    /// Access the ciphertext bytes.
    pub fn ciphertext(&self) -> &[u8] {
        todo!()
    }

    /// Access the key (who it's encrypted for).
    pub fn key(&self) -> &K {
        todo!()
    }
}

/// Visibility layer encoding. Sign, encrypt, decrypt.
/// Self threads through as real data — the signer in Signed,
/// the recipient in Encrypted.
pub trait Keys: Sized + Clone {
    /// Sign a fragment. The signature proves authorship.
    fn sign<E>(&self, fragment: Fragment<E>) -> Signed<Self, Fragment<E>>;

    /// Encrypt a fragment. The result is opaque bytes.
    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> Encrypted<Self>;

    /// Decrypt an encrypted fragment.
    fn decrypt<E: Decode>(&self, encrypted: &Encrypted<Self>) -> Fragment<E>;
}

/// No-op keys: empty signatures, no encryption.
/// For testing and unencrypted contexts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlainKeys;

impl Keys for PlainKeys {
    fn sign<E>(&self, _fragment: Fragment<E>) -> Signed<Self, Fragment<E>> {
        todo!()
    }

    fn encrypt<E: Encode>(&self, _fragment: Fragment<E>) -> Encrypted<Self> {
        todo!()
    }

    fn decrypt<E: Decode>(&self, _encrypted: &Encrypted<Self>) -> Fragment<E> {
        todo!()
    }
}
