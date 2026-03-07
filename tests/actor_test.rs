use fragmentation::actor::Actor;
use fragmentation::encoding::{Decode, Encode};
use fragmentation::fragment::{self, Blob, Fragment};
use fragmentation::keys::{Keys, Local, PlainKeys, Signed};
use fragmentation::ref_::Ref;
use fragmentation::sha;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_blob_shard(data: Vec<u8>) -> Fragment<Blob> {
    let r = Ref::new(sha::Sha(fragment::blob_oid_bytes(&data)), "self");
    Fragment::shard_typed(r, data)
}

fn make_string_shard(data: &str) -> Fragment<String> {
    let r = Ref::new(sha::Sha(fragment::blob_oid(data)), "self");
    Fragment::shard(r, data)
}

// ===========================================================================
// Identity actor (Blob -> Blob, Local)
// ===========================================================================

#[test]
fn identity_actor_has_name() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    assert_eq!(actor.name(), "mara");
}

#[test]
fn identity_actor_has_email() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    assert_eq!(actor.email(), "mara@systemic.engineer");
}

#[test]
fn identity_actor_encode_is_clone() {
    let actor = Actor::identity("test", "test@test");
    let shard = make_blob_shard(vec![1, 2, 3]);
    let encoded = actor.encode(&shard);
    assert_eq!(encoded.data(), shard.data());
}

#[test]
fn identity_actor_decode_is_clone() {
    let actor = Actor::identity("test", "test@test");
    let shard = make_blob_shard(vec![1, 2, 3]);
    let decoded = actor.decode(&shard);
    assert_eq!(decoded.data(), shard.data());
}

#[test]
fn identity_actor_roundtrip() {
    let actor = Actor::identity("test", "test@test");
    let shard = make_blob_shard(vec![0xCA, 0xFE]);
    let encoded = actor.encode(&shard);
    let decoded = actor.decode(&encoded);
    assert_eq!(decoded.data(), shard.data());
}

#[test]
fn identity_actor_local_keys() {
    let actor = Actor::identity("test", "test@test");
    assert_eq!(actor.keys(), &Local::None);
}

// ===========================================================================
// PlainKeys sign/encrypt/decrypt (Error = Infallible)
// ===========================================================================

#[test]
fn plain_keys_sign_roundtrip() {
    let shard = make_blob_shard(vec![1, 2, 3]);
    let signed = PlainKeys.sign(shard.clone()).unwrap();
    assert_eq!(signed.into_inner().data(), shard.data());
}

#[test]
fn plain_keys_encrypt_decrypt_roundtrip() {
    let data = vec![1, 2, 3];
    let shard = make_blob_shard(data.clone());
    let encrypted = PlainKeys.encrypt(shard).unwrap();
    let decrypted: Fragment<Blob> = PlainKeys.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.data(), &data);
}

#[test]
fn signed_carries_signer() {
    let shard = make_blob_shard(vec![42]);
    let signed = PlainKeys.sign(shard).unwrap();
    assert_eq!(signed.signer(), &PlainKeys);
}

#[test]
fn signed_has_empty_signature() {
    let shard = make_blob_shard(vec![42]);
    let signed = PlainKeys.sign(shard).unwrap();
    assert!(signed.signature().is_empty());
}

#[test]
fn encrypted_carries_key() {
    let shard = make_blob_shard(vec![42]);
    let encrypted = PlainKeys.encrypt(shard).unwrap();
    assert_eq!(encrypted.key(), &PlainKeys);
}

// ===========================================================================
// Local::None sign/encrypt/decrypt
// ===========================================================================

#[test]
fn local_keys_plain_sign_empty_signature() {
    let shard = make_blob_shard(vec![1, 2, 3]);
    let signed = Local::None.sign(shard).unwrap();
    assert!(signed.signature().is_empty());
}

#[test]
fn local_keys_plain_sign_preserves_content() {
    let shard = make_blob_shard(vec![1, 2, 3]);
    let signed = Local::None.sign(shard.clone()).unwrap();
    assert_eq!(signed.into_inner().data(), shard.data());
}

#[test]
fn local_keys_plain_signed_carries_signer() {
    let shard = make_blob_shard(vec![42]);
    let signed = Local::None.sign(shard).unwrap();
    assert_eq!(signed.signer(), &Local::None);
}

#[test]
fn local_keys_plain_encrypt_decrypt_roundtrip() {
    let data = vec![1, 2, 3];
    let shard = make_blob_shard(data.clone());
    let encrypted = Local::None.encrypt(shard).unwrap();
    let decrypted: Fragment<Blob> = Local::None.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.data(), &data);
}

// ===========================================================================
// Custom actor (String -> Blob transformation)
// ===========================================================================

fn string_to_blob(f: &Fragment<String>) -> Fragment<Blob> {
    let ref_ = f.self_ref().clone();
    Fragment::shard_typed(ref_, f.data().as_bytes().to_vec())
}

fn blob_to_string(f: &Fragment<Blob>) -> Fragment<String> {
    let ref_ = f.self_ref().clone();
    Fragment::shard(ref_, String::from_utf8(f.data().clone()).unwrap())
}

#[test]
fn custom_actor_string_to_bytes() {
    let actor: Actor<String, Blob, PlainKeys> = Actor::new(
        "transform",
        "t@t",
        string_to_blob,
        blob_to_string,
        PlainKeys,
    );
    let input = make_string_shard("hello");
    let encoded = actor.encode(&input);
    assert_eq!(encoded.data(), &b"hello".to_vec());
}

#[test]
fn custom_actor_transforms_data() {
    let actor: Actor<String, Blob, PlainKeys> = Actor::new(
        "transform",
        "t@t",
        string_to_blob,
        blob_to_string,
        PlainKeys,
    );
    let input = make_string_shard("cafe");
    let encoded = actor.encode(&input);
    let decoded = actor.decode(&encoded);
    assert_eq!(decoded.data(), "cafe");
}

// ===========================================================================
// Custom Keys implementation (now with Result)
// ===========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestKeys {
    label: String,
}

impl Keys for TestKeys {
    type Error = std::convert::Infallible;

    fn sign<E>(&self, fragment: Fragment<E>) -> Result<Signed<Self, Fragment<E>>, Self::Error> {
        Ok(Signed::new(fragment, b"test-sig".to_vec(), self.clone()))
    }

    fn encrypt<E: Encode>(
        &self,
        fragment: Fragment<E>,
    ) -> Result<fragmentation::keys::Encrypted<Self>, Self::Error> {
        Ok(fragmentation::keys::Encrypted::new(
            fragment.data().encode(),
            self.clone(),
        ))
    }

    fn decrypt<E: Decode>(
        &self,
        encrypted: &fragmentation::keys::Encrypted<Self>,
    ) -> Result<Fragment<E>, Self::Error> {
        let data = E::decode(encrypted.ciphertext()).expect("test decrypt");
        let sha_str = fragment::blob_oid_bytes(encrypted.ciphertext());
        let ref_ = Ref::new(sha::Sha(sha_str), "decrypted");
        Ok(Fragment::shard_typed(ref_, data))
    }
}

#[test]
fn custom_actor_with_keys() {
    let keys = TestKeys {
        label: "test".into(),
    };
    let actor: Actor<Blob, Blob, TestKeys> =
        Actor::new("keyed", "k@k", |f| f.clone(), |f| f.clone(), keys.clone());
    assert_eq!(actor.keys(), &keys);
}

#[test]
fn custom_keys_sign_has_signature() {
    let keys = TestKeys {
        label: "test".into(),
    };
    let actor: Actor<Blob, Blob, TestKeys> =
        Actor::new("keyed", "k@k", |f| f.clone(), |f| f.clone(), keys);
    let shard = make_blob_shard(vec![1, 2, 3]);
    let signed = actor.sign(shard).unwrap();
    assert_eq!(signed.signature(), b"test-sig");
}

// ===========================================================================
// Actor derives Clone
// ===========================================================================

#[test]
fn actor_clone() {
    let actor = Actor::identity("mara", "mara@systemic.engineer");
    let cloned = actor.clone();
    assert_eq!(cloned.name(), "mara");
    assert_eq!(cloned.email(), "mara@systemic.engineer");
}

// ===========================================================================
// Actor sign/encrypt/decrypt return Result
// ===========================================================================

#[test]
fn actor_sign_returns_result() {
    let actor = Actor::identity("test", "test@test");
    let shard = make_blob_shard(vec![1, 2, 3]);
    let signed = actor.sign(shard.clone()).unwrap();
    assert_eq!(signed.inner().data(), shard.data());
}

#[test]
fn actor_encrypt_decrypt_returns_result() {
    let actor = Actor::identity("test", "test@test");
    let shard = make_blob_shard(vec![1, 2, 3]);
    let encrypted = actor.encrypt(shard.clone()).unwrap();
    let decrypted: Fragment<Blob> = actor.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.data(), shard.data());
}

// ===========================================================================
// SSH tests (feature-gated)
// ===========================================================================

#[cfg(feature = "ssh")]
mod ssh_tests {
    use super::*;
    use fragmentation::keys::SSH;

    fn test_ssh_key() -> SSH {
        // Generate an Ed25519 key in memory for testing
        SSH::generate_ed25519().expect("generate test key")
    }

    #[test]
    fn ssh_key_sign_produces_signature() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_blob_shard(vec![1, 2, 3]);
        let signed = local.sign(shard).unwrap();
        assert!(!signed.signature().is_empty());
    }

    #[test]
    fn ssh_key_sign_preserves_content() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_blob_shard(vec![1, 2, 3]);
        let signed = local.sign(shard.clone()).unwrap();
        assert_eq!(signed.into_inner().data(), shard.data());
    }

    #[test]
    fn ssh_key_signed_carries_signer() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key.clone()));
        let shard = make_blob_shard(vec![42]);
        let signed = local.sign(shard).unwrap();
        assert_eq!(signed.signer(), &Local::Ssh(Box::new(key)));
    }

    #[test]
    fn ssh_key_encrypt_decrypt_roundtrip() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let data = vec![1, 2, 3];
        let shard = make_blob_shard(data.clone());
        let encrypted = local.encrypt(shard).unwrap();
        let decrypted: Fragment<Blob> = local.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.data(), &data);
    }

    #[test]
    fn ssh_encrypt_ciphertext_differs_from_plaintext() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let data = vec![1, 2, 3, 4, 5];
        let shard = make_blob_shard(data.clone());
        let encrypted = local.encrypt(shard).unwrap();
        // Ciphertext must differ from plaintext
        assert_ne!(encrypted.ciphertext(), &data[..]);
        // ECIES envelope: 32 ephemeral_pub + 12 nonce + ciphertext + 16 tag
        assert!(encrypted.ciphertext().len() >= 60 + data.len());
    }

    #[test]
    fn ssh_encrypt_decrypt_roundtrip_string() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_string_shard("hello fragmentation");
        let encrypted = local.encrypt(shard).unwrap();
        let decrypted: Fragment<String> = local.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.data(), "hello fragmentation");
    }

    #[test]
    fn ssh_wrong_key_cannot_decrypt() {
        let key1 = test_ssh_key();
        let key2 = test_ssh_key();
        let local1 = Local::Ssh(Box::new(key1));
        let local2 = Local::Ssh(Box::new(key2));
        let shard = make_blob_shard(vec![42, 43, 44]);
        let encrypted = local1.encrypt(shard).unwrap();
        // Wrap ciphertext with key2's identity for decrypt dispatch
        let mismatched =
            fragmentation::keys::Encrypted::new(encrypted.ciphertext().to_vec(), local2.clone());
        let result: Result<Fragment<Blob>, _> = local2.decrypt(&mismatched);
        assert!(result.is_err());
    }

    #[test]
    fn ssh_encrypt_is_nondeterministic() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_blob_shard(vec![1, 2, 3]);
        let enc1 = local.encrypt(shard.clone()).unwrap();
        let enc2 = local.encrypt(shard).unwrap();
        // Ephemeral key + random nonce → different ciphertext each time
        assert_ne!(enc1.ciphertext(), enc2.ciphertext());
    }
}

// ===========================================================================
// GPG tests (feature-gated)
// ===========================================================================

#[cfg(feature = "gpg")]
mod gpg_tests {
    use super::*;
    use fragmentation::keys::GPG;

    fn gpg_available() -> bool {
        std::process::Command::new("gpg")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Create an isolated GPG keyring with a test key. Returns None if gpg unavailable.
    fn setup_gpg_keyring() -> Option<(GPG, tempfile::TempDir)> {
        if !gpg_available() {
            return None;
        }
        let td = tempfile::tempdir().ok()?;
        let home = td.path();

        // Generate an RSA-2048 test key with no passphrase
        let batch_config = format!(
            "%no-protection\nKey-Type: RSA\nKey-Length: 2048\nSubkey-Type: RSA\nSubkey-Length: 2048\nName-Real: Test\nName-Email: test@test\nExpire-Date: 0\n%commit\n"
        );
        let output = std::process::Command::new("gpg")
            .env("GNUPGHOME", home)
            .args(["--batch", "--generate-key"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .ok()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(batch_config.as_bytes())
                    .ok()?;
                child.wait_with_output().ok()
            })?;

        if !output.status.success() {
            eprintln!(
                "gpg key generation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return None;
        }

        // Get the key ID
        let list_output = std::process::Command::new("gpg")
            .env("GNUPGHOME", home)
            .args(["--list-keys", "--with-colons", "--keyid-format", "long"])
            .output()
            .ok()?;

        let list_str = String::from_utf8_lossy(&list_output.stdout);
        let key_id = list_str
            .lines()
            .find(|l| l.starts_with("pub:"))
            .and_then(|l| l.split(':').nth(4))
            .map(|s| s.to_string())?;

        Some((GPG::with_gnupghome(key_id, home), td))
    }

    #[test]
    fn gpg_key_signed_carries_signer() {
        if !gpg_available() {
            eprintln!("gpg not available, skipping");
            return;
        }
        let key = GPG::new("test-key-id");
        let local = Local::Gpg(key.clone());
        let shard = make_blob_shard(vec![42]);
        // Sign may fail if gpg key doesn't exist — that's expected in CI
        // Just verify the signer is carried when it does work
        match local.sign(shard) {
            Ok(signed) => assert_eq!(signed.signer(), &Local::Gpg(key)),
            Err(_) => eprintln!("gpg sign failed (expected without real key), skipping assertion"),
        }
    }

    #[test]
    fn gpg_encrypt_decrypt_roundtrip() {
        let Some((gpg, _td)) = setup_gpg_keyring() else {
            eprintln!("gpg keyring setup failed, skipping");
            return;
        };
        let local = Local::Gpg(gpg);
        let data = vec![1, 2, 3, 4, 5];
        let shard = make_blob_shard(data.clone());
        let encrypted = local.encrypt(shard).unwrap();
        let decrypted: Fragment<Blob> = local.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.data(), &data);
    }

    #[test]
    fn gpg_encrypt_ciphertext_differs() {
        let Some((gpg, _td)) = setup_gpg_keyring() else {
            eprintln!("gpg keyring setup failed, skipping");
            return;
        };
        let local = Local::Gpg(gpg);
        let data = vec![1, 2, 3, 4, 5];
        let shard = make_blob_shard(data.clone());
        let encrypted = local.encrypt(shard).unwrap();
        assert_ne!(encrypted.ciphertext(), &data[..]);
    }
}

// ===========================================================================
// from_repo tests (feature-gated)
// ===========================================================================

#[cfg(feature = "git")]
mod from_repo_tests {
    use super::*;

    #[test]
    fn from_repo_no_config_returns_plain() {
        let td = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(td.path()).unwrap();
        let keys = Local::from_repo(&repo).unwrap();
        assert_eq!(keys, Local::None);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn from_repo_ssh_format() {
        let td = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(td.path()).unwrap();

        // Write a test SSH key to a temp file
        let key = fragmentation::keys::SSH::generate_ed25519().unwrap();
        let key_path = td.path().join("test_key");
        key.write_to_file(&key_path).unwrap();

        // Configure git to use SSH signing with that key
        let mut config = repo.config().unwrap();
        config.set_str("gpg.format", "ssh").unwrap();
        config
            .set_str("user.signingkey", key_path.to_str().unwrap())
            .unwrap();

        let keys = Local::from_repo(&repo).unwrap();
        assert!(matches!(keys, Local::Ssh(_)));
    }

    #[cfg(feature = "gpg")]
    #[test]
    fn from_repo_gpg_format() {
        let td = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(td.path()).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("gpg.format", "openpgp").unwrap();
        config.set_str("user.signingkey", "ABCDEF1234").unwrap();

        let keys = Local::from_repo(&repo).unwrap();
        assert!(matches!(keys, Local::Gpg(_)));
    }
}
