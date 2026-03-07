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
// Local — maps what the local machine has
// ===========================================================================

/// Error type for Local operations.
#[derive(Clone, Debug)]
pub enum LocalError {
    Decode(String),
    #[cfg(feature = "ssh")]
    Ssh(String),
    #[cfg(feature = "gpg")]
    Gpg(String),
}

impl fmt::Display for LocalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalError::Decode(msg) => write!(f, "decode error: {}", msg),
            #[cfg(feature = "ssh")]
            LocalError::Ssh(msg) => write!(f, "ssh error: {}", msg),
            #[cfg(feature = "gpg")]
            LocalError::Gpg(msg) => write!(f, "gpg error: {}", msg),
        }
    }
}

/// The locally available key types.
/// Plain = no signing. Ssh/Gpg = real signing, feature-gated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Local {
    None,
    #[cfg(feature = "ssh")]
    Ssh(Box<SSH>),
    #[cfg(feature = "gpg")]
    Gpg(GPG),
}

impl Keys for Local {
    type Error = LocalError;

    fn sign<E>(&self, fragment: Fragment<E>) -> Result<Signed<Self, Fragment<E>>, Self::Error> {
        match self {
            Local::None => Ok(Signed::new(fragment, vec![], Local::None)),
            #[cfg(feature = "ssh")]
            Local::Ssh(ssh_key) => {
                let sha_bytes = fragment.self_ref().sha.0.as_bytes();
                let signature = ssh_key.sign_bytes(sha_bytes)?;
                Ok(Signed::new(fragment, signature, self.clone()))
            }
            #[cfg(feature = "gpg")]
            Local::Gpg(gpg_key) => {
                let sha_bytes = fragment.self_ref().sha.0.as_bytes();
                let signature = gpg_key.sign_bytes(sha_bytes)?;
                Ok(Signed::new(fragment, signature, self.clone()))
            }
        }
    }

    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> Result<Encrypted<Self>, Self::Error> {
        let plaintext = fragment.data().encode();
        let ciphertext = match self {
            Local::None => plaintext,
            #[cfg(feature = "ssh")]
            Local::Ssh(_) => todo!("SSH ECIES encryption"),
            #[cfg(feature = "gpg")]
            Local::Gpg(_) => todo!("GPG subprocess encryption"),
        };
        Ok(Encrypted::new(ciphertext, self.clone()))
    }

    fn decrypt<E: Decode>(&self, encrypted: &Encrypted<Self>) -> Result<Fragment<E>, Self::Error> {
        let plaintext = match self {
            Local::None => encrypted.ciphertext.clone(),
            #[cfg(feature = "ssh")]
            Local::Ssh(_) => todo!("SSH ECIES decryption"),
            #[cfg(feature = "gpg")]
            Local::Gpg(_) => todo!("GPG subprocess decryption"),
        };
        let data = E::decode(&plaintext).map_err(|e| LocalError::Decode(format!("{}", e)))?;
        let sha = Sha(fragment::blob_oid_bytes(&plaintext));
        let ref_ = Ref::new(sha, "decrypted");
        Ok(Fragment::shard_typed(ref_, data))
    }
}

// ===========================================================================
// SSH key (behind `ssh` feature)
// ===========================================================================

#[cfg(feature = "ssh")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SSH {
    key: ssh_key::PrivateKey,
}

#[cfg(feature = "ssh")]
impl SSH {
    /// Load an SSH private key from a file path.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ssh_key::Error> {
        let key = ssh_key::PrivateKey::read_openssh_file(path.as_ref())?;
        Ok(SSH { key })
    }

    /// Generate an Ed25519 key in memory (for testing).
    pub fn generate_ed25519() -> Result<Self, ssh_key::Error> {
        let key = ssh_key::PrivateKey::random(
            &mut ssh_key::rand_core::OsRng,
            ssh_key::Algorithm::Ed25519,
        )?;
        Ok(SSH { key })
    }

    /// Write the private key to a file (for testing).
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), ssh_key::Error> {
        self.key
            .write_openssh_file(path.as_ref(), ssh_key::LineEnding::LF)
    }

    /// Sign raw bytes, returning the PEM-encoded SSH signature.
    fn sign_bytes(&self, data: &[u8]) -> Result<Vec<u8>, LocalError> {
        let sig = self
            .key
            .sign("fragmentation", ssh_key::HashAlg::Sha256, data)
            .map_err(|e| LocalError::Ssh(format!("{}", e)))?;
        let pem = sig
            .to_pem(ssh_key::LineEnding::LF)
            .map_err(|e| LocalError::Ssh(format!("{}", e)))?;
        Ok(pem.into_bytes())
    }

    /// Derive X25519 static secret from Ed25519 seed.
    fn x25519_secret(&self) -> Result<x25519_dalek::StaticSecret, LocalError> {
        todo!("Ed25519 → X25519 conversion")
    }

    /// Derive X25519 public key from the static secret.
    fn x25519_public(&self) -> Result<x25519_dalek::PublicKey, LocalError> {
        todo!("X25519 public key derivation")
    }

    /// ECIES encrypt: ephemeral X25519 + HKDF-SHA256 + ChaCha20-Poly1305.
    fn encrypt_bytes(&self, _plaintext: &[u8]) -> Result<Vec<u8>, LocalError> {
        todo!("SSH ECIES encryption")
    }

    /// ECIES decrypt: parse wire format, ECDH with static secret, HKDF, AEAD decrypt.
    fn decrypt_bytes(&self, _data: &[u8]) -> Result<Vec<u8>, LocalError> {
        todo!("SSH ECIES decryption")
    }
}

// ===========================================================================
// GPG key (behind `gpg` feature)
// ===========================================================================

#[cfg(feature = "gpg")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GPG {
    key_id: String,
    gnupghome: Option<std::path::PathBuf>,
}

#[cfg(feature = "gpg")]
impl GPG {
    pub fn new(key_id: impl Into<String>) -> Self {
        GPG {
            key_id: key_id.into(),
            gnupghome: None,
        }
    }

    /// Constructor with custom GNUPGHOME for test isolation.
    pub fn with_gnupghome(
        key_id: impl Into<String>,
        gnupghome: impl Into<std::path::PathBuf>,
    ) -> Self {
        GPG {
            key_id: key_id.into(),
            gnupghome: Some(gnupghome.into()),
        }
    }

    /// Build a gpg Command with optional GNUPGHOME.
    fn gpg_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new("gpg");
        if let Some(ref home) = self.gnupghome {
            cmd.env("GNUPGHOME", home);
        }
        cmd
    }

    /// Sign raw bytes via gpg CLI, returning the detached signature.
    fn sign_bytes(&self, data: &[u8]) -> Result<Vec<u8>, LocalError> {
        use std::io::Write;
        use std::process::Stdio;

        let mut child = self
            .gpg_command()
            .args([
                "--detach-sign",
                "--armor",
                "-u",
                &self.key_id,
                "--batch",
                "--yes",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| LocalError::Gpg(format!("failed to spawn gpg: {}", e)))?;

        child
            .stdin
            .take()
            .unwrap()
            .write_all(data)
            .map_err(|e| LocalError::Gpg(format!("failed to write to gpg stdin: {}", e)))?;

        let output = child
            .wait_with_output()
            .map_err(|e| LocalError::Gpg(format!("gpg failed: {}", e)))?;

        if !output.status.success() {
            return Err(LocalError::Gpg(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(output.stdout)
    }

    /// Encrypt raw bytes via gpg CLI subprocess.
    fn encrypt_bytes(&self, _plaintext: &[u8]) -> Result<Vec<u8>, LocalError> {
        todo!("GPG subprocess encryption")
    }

    /// Decrypt raw bytes via gpg CLI subprocess.
    fn decrypt_bytes(&self, _ciphertext: &[u8]) -> Result<Vec<u8>, LocalError> {
        todo!("GPG subprocess decryption")
    }
}

// ===========================================================================
// from_repo (behind `git` feature)
// ===========================================================================

#[cfg(feature = "git")]
impl Local {
    /// Detect signing configuration from a git repository.
    /// Reads gpg.format and user.signingkey from git config.
    pub fn from_repo(repo: &git2::Repository) -> Result<Self, LocalError> {
        let config = repo
            .config()
            .and_then(|c| c.open_level(git2::ConfigLevel::Local))
            .map_err(|e| LocalError::Decode(format!("failed to read git config: {}", e)))?;

        let format = config.get_string("gpg.format").unwrap_or_default();
        let signing_key = config.get_string("user.signingkey").ok();

        match (format.as_str(), signing_key) {
            #[cfg(feature = "ssh")]
            ("ssh", Some(key_path)) => {
                let ssh_key =
                    SSH::from_path(&key_path).map_err(|e| LocalError::Ssh(format!("{}", e)))?;
                Ok(Local::Ssh(Box::new(ssh_key)))
            }
            #[cfg(feature = "gpg")]
            ("openpgp" | "", Some(key_id)) => Ok(Local::Gpg(GPG::new(key_id))),
            _ => Ok(Local::None),
        }
    }
}
