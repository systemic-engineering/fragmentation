use crate::encoding::{Decode, Encode};
use crate::fragment::{self, Fragment};
use crate::ref_::Ref;
use crate::sha::Sha;

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
        &self.inner
    }

    /// Access the signature bytes.
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    /// Access the signer.
    pub fn signer(&self) -> &K {
        &self.signer
    }

    /// Consume the wrapper, returning the inner value.
    pub fn into_inner(self) -> T {
        self.inner
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
        &self.ciphertext
    }

    /// Access the key (who it's encrypted for).
    pub fn key(&self) -> &K {
        &self.key
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
    fn sign<E>(&self, fragment: Fragment<E>) -> Signed<Self, Fragment<E>> {
        Signed::new(fragment, vec![], PlainKeys)
    }

    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> Encrypted<Self> {
        Encrypted::new(fragment.data().encode(), PlainKeys)
    }

    fn decrypt<E: Decode>(&self, encrypted: &Encrypted<Self>) -> Fragment<E> {
        let data = E::decode(&encrypted.ciphertext).expect("PlainKeys: invalid data");
        let sha = Sha(fragment::blob_oid_bytes(&encrypted.ciphertext));
        let ref_ = Ref::new(sha, "decrypted");
        Fragment::shard_typed(ref_, data)
    }
}
