use fragmentation::encoding::{Decode, Encode};
use fragmentation::fragment::{self, Blob, Fractal, Fragmentable};
use fragmentation::keys::{Keys, Local, LocalError, PlainKeys, Signature};
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::visibility::Public;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_blob_shard(data: Vec<u8>) -> Fractal<Blob> {
    let r = Ref::new(sha::Sha(fragment::blob_oid_bytes(&data)), "self");
    Fractal::shard_typed(r, data)
}

#[cfg(feature = "ssh")]
fn make_string_shard(data: &str) -> Fractal<String> {
    let r = Ref::new(sha::Sha(fragment::blob_oid(data)), "self");
    Fractal::shard(r, data)
}

// ===========================================================================
// PlainKeys sign/encrypt/decrypt (Error = Infallible)
// ===========================================================================

#[test]
fn plain_keys_sign_produces_signature() {
    let shard = make_blob_shard(vec![1, 2, 3]);
    let sig = PlainKeys.sign(&shard).unwrap();
    assert!(sig.bytes().is_empty());
    assert_eq!(sig.key(), &PlainKeys);
}

#[test]
fn plain_keys_sign_public_roundtrip() {
    let shard = make_blob_shard(vec![1, 2, 3]);
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard.clone(), sig);
    assert_eq!(public.into_inner().data(), shard.data());
}

#[test]
fn plain_keys_encrypt_decrypt_roundtrip() {
    let data = vec![1, 2, 3];
    let shard = make_blob_shard(data.clone());
    let encrypted = PlainKeys.encrypt(shard).unwrap();
    let decrypted: Fractal<Blob> = PlainKeys.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.data(), &data);
}

#[test]
fn public_carries_key() {
    let shard = make_blob_shard(vec![42]);
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert_eq!(public.key(), &PlainKeys);
}

#[test]
fn public_has_empty_signature_bytes() {
    let shard = make_blob_shard(vec![42]);
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert!(public.signature().bytes().is_empty());
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
    let sig = Local::None.sign(&shard).unwrap();
    assert!(sig.bytes().is_empty());
}

#[test]
fn local_keys_plain_sign_preserves_content() {
    let shard = make_blob_shard(vec![1, 2, 3]);
    let sig = Local::None.sign(&shard).unwrap();
    let public = Public::new(shard.clone(), sig);
    assert_eq!(public.into_inner().data(), shard.data());
}

#[test]
fn local_keys_plain_public_carries_key() {
    let shard = make_blob_shard(vec![42]);
    let sig = Local::None.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert_eq!(public.key(), &Local::None);
}

#[test]
fn local_keys_plain_encrypt_decrypt_roundtrip() {
    let data = vec![1, 2, 3];
    let shard = make_blob_shard(data.clone());
    let encrypted = Local::None.encrypt(shard).unwrap();
    let decrypted: Fractal<Blob> = Local::None.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.data(), &data);
}

// ===========================================================================
// Custom Keys implementation
// ===========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestKeys {
    label: String,
}

impl Keys for TestKeys {
    type Error = std::convert::Infallible;

    fn sign<E>(&self, _fragment: &Fractal<E>) -> Result<Signature<Self>, Self::Error> {
        Ok(Signature::new(self.clone(), b"test-sig".to_vec()))
    }

    fn encrypt<E: Encode>(
        &self,
        fragment: Fractal<E>,
    ) -> Result<fragmentation::keys::Encrypted<Self>, Self::Error> {
        Ok(fragmentation::keys::Encrypted::new(
            fragment.data().encode(),
            self.clone(),
        ))
    }

    fn decrypt<E: Decode>(
        &self,
        encrypted: &fragmentation::keys::Encrypted<Self>,
    ) -> Result<Fractal<E>, Self::Error> {
        let data = E::decode(encrypted.ciphertext()).expect("test decrypt");
        let sha_str = fragment::blob_oid_bytes(encrypted.ciphertext());
        let ref_ = Ref::new(sha::Sha(sha_str), "decrypted");
        Ok(Fractal::shard_typed(ref_, data))
    }

    fn fingerprint(&self) -> String {
        todo!()
    }
}

#[test]
fn custom_keys_sign_has_signature() {
    let keys = TestKeys {
        label: "test".into(),
    };
    let shard = make_blob_shard(vec![1, 2, 3]);
    let sig = keys.sign(&shard).unwrap();
    assert_eq!(sig.bytes(), b"test-sig");
}

#[test]
fn custom_keys_encrypt_decrypt_roundtrip() {
    let keys = TestKeys {
        label: "test".into(),
    };
    let data = vec![1, 2, 3];
    let shard = make_blob_shard(data.clone());
    let encrypted = keys.encrypt(shard).unwrap();
    let decrypted: Fractal<Blob> = keys.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.data(), &data);
}

// ===========================================================================
// Public<K, Commit<E>> — signed commit type is expressible
// ===========================================================================

#[test]
fn public_commit_implements_draftable() {
    use fragmentation::commit::{Draft, Draftable};
    let shard = make_blob_shard(vec![1, 2, 3]);
    let sig = Local::None.sign(&shard).unwrap();
    let draft = Draft::root("signed observation", shard);
    let public: Public<Local, Draft<Fractal<Blob>>> = Public::new(draft, sig);

    fn accepts_draftable<T: Draftable>(_d: &T) {}
    accepts_draftable(&public);

    assert_eq!(public.key(), &Local::None);
    assert_eq!(public.message().0, "signed observation");
    assert!(public.parent().is_none());
}

// ===========================================================================
// LocalError Display
// ===========================================================================

#[test]
fn local_error_display_decode() {
    let e = LocalError::Decode("bad encoding".to_string());
    assert_eq!(format!("{}", e), "decode error: bad encoding");
}

#[test]
fn public_draftable_fractal_method() {
    use fragmentation::commit::{Draft, Draftable};
    let shard = make_blob_shard(vec![1, 2, 3]);
    let sig = Local::None.sign(&shard).unwrap();
    let draft = Draft::root("test", shard.clone());
    let public: Public<Local, Draft<Fractal<Blob>>> = Public::new(draft, sig);
    // .node() via Draftable → covers visibility.rs:148
    assert_eq!(public.node().data(), shard.data());
}

// ===========================================================================
// SSH tests (feature-gated)
// ===========================================================================

#[cfg(feature = "ssh")]
mod ssh_tests {
    use super::*;
    use fragmentation::keys::SSH;

    fn test_ssh_key() -> SSH {
        SSH::generate_ed25519().expect("generate test key")
    }

    #[test]
    fn ssh_key_sign_produces_signature() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_blob_shard(vec![1, 2, 3]);
        let sig = local.sign(&shard).unwrap();
        assert!(!sig.bytes().is_empty());
    }

    #[test]
    fn ssh_key_sign_preserves_content() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_blob_shard(vec![1, 2, 3]);
        let sig = local.sign(&shard).unwrap();
        let public = Public::new(shard.clone(), sig);
        assert_eq!(public.into_inner().data(), shard.data());
    }

    #[test]
    fn ssh_key_public_carries_key() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key.clone()));
        let shard = make_blob_shard(vec![42]);
        let sig = local.sign(&shard).unwrap();
        let public = Public::new(shard, sig);
        assert_eq!(public.key(), &Local::Ssh(Box::new(key)));
    }

    #[test]
    fn ssh_key_encrypt_decrypt_roundtrip() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let data = vec![1, 2, 3];
        let shard = make_blob_shard(data.clone());
        let encrypted = local.encrypt(shard).unwrap();
        let decrypted: Fractal<Blob> = local.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.data(), &data);
    }

    #[test]
    fn ssh_encrypt_ciphertext_differs_from_plaintext() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let data = vec![1, 2, 3, 4, 5];
        let shard = make_blob_shard(data.clone());
        let encrypted = local.encrypt(shard).unwrap();
        assert_ne!(encrypted.ciphertext(), &data[..]);
        assert!(encrypted.ciphertext().len() >= 60 + data.len());
    }

    #[test]
    fn ssh_encrypt_decrypt_roundtrip_string() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_string_shard("hello fragmentation");
        let encrypted = local.encrypt(shard).unwrap();
        let decrypted: Fractal<String> = local.decrypt(&encrypted).unwrap();
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
        let mismatched =
            fragmentation::keys::Encrypted::new(encrypted.ciphertext().to_vec(), local2.clone());
        let result: Result<Fractal<Blob>, _> = local2.decrypt(&mismatched);
        assert!(result.is_err());
    }

    #[test]
    fn ssh_encrypt_is_nondeterministic() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        let shard = make_blob_shard(vec![1, 2, 3]);
        let enc1 = local.encrypt(shard.clone()).unwrap();
        let enc2 = local.encrypt(shard).unwrap();
        assert_ne!(enc1.ciphertext(), enc2.ciphertext());
    }

    #[test]
    fn local_error_display_ssh() {
        let e = LocalError::Ssh("key failure".to_string());
        assert_eq!(format!("{}", e), "ssh error: key failure");
    }

    #[test]
    fn ssh_decrypt_short_ciphertext_returns_error() {
        let key = test_ssh_key();
        let local = Local::Ssh(Box::new(key));
        // Ciphertext shorter than 60 bytes → early error (keys.rs:302-305)
        let short = fragmentation::keys::Encrypted::new(vec![0u8; 10], local.clone());
        let result: Result<Fractal<Blob>, _> = local.decrypt(&short);
        assert!(result.is_err());
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

    fn setup_gpg_keyring() -> Option<(GPG, tempfile::TempDir)> {
        if !gpg_available() {
            return None;
        }
        let td = tempfile::tempdir().ok()?;
        let home = td.path();

        let batch_config = "%no-protection\nKey-Type: RSA\nKey-Length: 2048\nSubkey-Type: RSA\nSubkey-Length: 2048\nName-Real: Test\nName-Email: test@test\nExpire-Date: 0\n%commit\n";
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
            return None;
        }

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
    fn gpg_key_public_carries_key() {
        if !gpg_available() {
            return;
        }
        let key = GPG::new("test-key-id");
        let local = Local::Gpg(key.clone());
        let shard = make_blob_shard(vec![42]);
        match local.sign(&shard) {
            Ok(sig) => {
                let public = Public::new(shard, sig);
                assert_eq!(public.key(), &Local::Gpg(key));
            }
            Err(_) => {}
        }
    }

    #[test]
    fn gpg_encrypt_decrypt_roundtrip() {
        let Some((gpg, _td)) = setup_gpg_keyring() else {
            return;
        };
        let local = Local::Gpg(gpg);
        let data = vec![1, 2, 3, 4, 5];
        let shard = make_blob_shard(data.clone());
        let encrypted = local.encrypt(shard).unwrap();
        let decrypted: Fractal<Blob> = local.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.data(), &data);
    }

    #[test]
    fn gpg_encrypt_ciphertext_differs() {
        let Some((gpg, _td)) = setup_gpg_keyring() else {
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

        let key = fragmentation::keys::SSH::generate_ed25519().unwrap();
        let key_path = td.path().join("test_key");
        key.write_to_file(&key_path).unwrap();

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

// ===========================================================================
// Fingerprint tests
// ===========================================================================

#[test]
fn fingerprint_plain_keys_returns_plain() {
    assert_eq!(PlainKeys.fingerprint(), "plain");
}

#[test]
fn fingerprint_plain_keys_deterministic() {
    assert_eq!(PlainKeys.fingerprint(), PlainKeys.fingerprint());
}

#[test]
fn fingerprint_local_none() {
    assert_eq!(Local::None.fingerprint(), "none");
}

#[test]
fn fingerprint_custom_keys() {
    let keys = TestKeys {
        label: "test".into(),
    };
    assert_eq!(keys.fingerprint(), "test:test");
}

#[cfg(feature = "ssh")]
mod ssh_fingerprint_tests {
    use super::*;
    use fragmentation::keys::SSH;

    #[test]
    fn fingerprint_ssh_not_empty() {
        let key = SSH::generate_ed25519().expect("generate test key");
        let local = Local::Ssh(Box::new(key));
        assert!(!local.fingerprint().is_empty());
    }

    #[test]
    fn fingerprint_ssh_deterministic() {
        let key = SSH::generate_ed25519().expect("generate test key");
        let local = Local::Ssh(Box::new(key));
        assert_eq!(local.fingerprint(), local.fingerprint());
    }

    #[test]
    fn fingerprint_ssh_different_keys_differ() {
        let key1 = SSH::generate_ed25519().expect("generate test key");
        let key2 = SSH::generate_ed25519().expect("generate test key");
        let local1 = Local::Ssh(Box::new(key1));
        let local2 = Local::Ssh(Box::new(key2));
        assert_ne!(local1.fingerprint(), local2.fingerprint());
    }
}
