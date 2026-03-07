use std::convert::Infallible;
use std::fmt;

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
    type Error: fmt::Display + fmt::Debug;

    /// Sign a fragment. The signature proves authorship.
    fn sign<E>(&self, fragment: Fragment<E>) -> Result<Signed<Self, Fragment<E>>, Self::Error>;

    /// Encrypt a fragment. The result is opaque bytes.
    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> Result<Encrypted<Self>, Self::Error>;

    /// Decrypt an encrypted fragment.
    fn decrypt<E: Decode>(&self, encrypted: &Encrypted<Self>) -> Result<Fragment<E>, Self::Error>;
}

/// No-op keys: empty signatures, no encryption.
/// For testing and unencrypted contexts.
/// Error = Infallible — plain operations cannot fail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlainKeys;

impl Keys for PlainKeys {
    type Error = Infallible;

    fn sign<E>(&self, fragment: Fragment<E>) -> Result<Signed<Self, Fragment<E>>, Self::Error> {
        Ok(Signed::new(fragment, vec![], PlainKeys))
    }

    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> Result<Encrypted<Self>, Self::Error> {
        Ok(Encrypted::new(fragment.data().encode(), PlainKeys))
    }

    fn decrypt<E: Decode>(&self, encrypted: &Encrypted<Self>) -> Result<Fragment<E>, Self::Error> {
        let data = E::decode(&encrypted.ciphertext).expect("PlainKeys: invalid data");
        let sha = Sha(fragment::blob_oid_bytes(&encrypted.ciphertext));
        let ref_ = Ref::new(sha, "decrypted");
        Ok(Fragment::shard_typed(ref_, data))
    }
}

// ===========================================================================
// LocalKeys — maps what the local machine has
// ===========================================================================

/// Error type for LocalKeys operations.
#[derive(Clone, Debug)]
pub enum LocalKeysError {
    Decode(String),
    #[cfg(feature = "ssh")]
    Ssh(String),
    #[cfg(feature = "gpg")]
    Gpg(String),
}

impl fmt::Display for LocalKeysError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalKeysError::Decode(msg) => write!(f, "decode error: {}", msg),
            #[cfg(feature = "ssh")]
            LocalKeysError::Ssh(msg) => write!(f, "ssh error: {}", msg),
            #[cfg(feature = "gpg")]
            LocalKeysError::Gpg(msg) => write!(f, "gpg error: {}", msg),
        }
    }
}

/// The locally available key types.
/// Plain = no signing. Ssh/Gpg = real signing, feature-gated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalKeys {
    Plain,
    #[cfg(feature = "ssh")]
    Ssh(SshKey),
    #[cfg(feature = "gpg")]
    Gpg(GpgKey),
}

impl Keys for LocalKeys {
    type Error = LocalKeysError;

    fn sign<E>(&self, fragment: Fragment<E>) -> Result<Signed<Self, Fragment<E>>, Self::Error> {
        todo!()
    }

    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> Result<Encrypted<Self>, Self::Error> {
        todo!()
    }

    fn decrypt<E: Decode>(&self, encrypted: &Encrypted<Self>) -> Result<Fragment<E>, Self::Error> {
        todo!()
    }
}

// ===========================================================================
// SSH key (behind `ssh` feature)
// ===========================================================================

#[cfg(feature = "ssh")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshKey {
    key: ssh_key::PrivateKey,
}

#[cfg(feature = "ssh")]
impl SshKey {
    /// Load an SSH private key from a file path.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ssh_key::Error> {
        todo!()
    }

    /// Generate an Ed25519 key in memory (for testing).
    pub fn generate_ed25519() -> Result<Self, ssh_key::Error> {
        todo!()
    }

    /// Write the private key to a file (for testing).
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), ssh_key::Error> {
        todo!()
    }
}

// ===========================================================================
// GPG key (behind `gpg` feature)
// ===========================================================================

#[cfg(feature = "gpg")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpgKey {
    key_id: String,
}

#[cfg(feature = "gpg")]
impl GpgKey {
    pub fn new(key_id: impl Into<String>) -> Self {
        GpgKey {
            key_id: key_id.into(),
        }
    }
}

// ===========================================================================
// from_repo (behind `git` feature)
// ===========================================================================

#[cfg(feature = "git")]
impl LocalKeys {
    /// Detect signing configuration from a git repository.
    /// Reads gpg.format and user.signingkey from git config.
    pub fn from_repo(repo: &git2::Repository) -> Result<Self, LocalKeysError> {
        todo!()
    }
}
