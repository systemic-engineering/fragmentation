use fragmentation::actor::Actor;
use fragmentation::encoding::{Decode, Encode};
use fragmentation::fragment::{self, Blob, Fragment};
use fragmentation::keys::{Keys, PlainKeys, Signed};
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
// Identity actor (Blob -> Blob, PlainKeys)
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
fn identity_actor_plain_keys() {
    let actor = Actor::identity("test", "test@test");
    assert_eq!(actor.keys(), &PlainKeys);
}

// ===========================================================================
// PlainKeys sign/encrypt/decrypt
// ===========================================================================

#[test]
fn plain_keys_sign_roundtrip() {
    let shard = make_blob_shard(vec![1, 2, 3]);
    let signed = PlainKeys.sign(shard.clone());
    assert_eq!(signed.into_inner().data(), shard.data());
}

#[test]
fn plain_keys_encrypt_decrypt_roundtrip() {
    let data = vec![1, 2, 3];
    let shard = make_blob_shard(data.clone());
    let encrypted = PlainKeys.encrypt(shard);
    let decrypted: Fragment<Blob> = PlainKeys.decrypt(&encrypted);
    assert_eq!(decrypted.data(), &data);
}

#[test]
fn signed_carries_signer() {
    let shard = make_blob_shard(vec![42]);
    let signed = PlainKeys.sign(shard);
    assert_eq!(signed.signer(), &PlainKeys);
}

#[test]
fn signed_has_empty_signature() {
    let shard = make_blob_shard(vec![42]);
    let signed = PlainKeys.sign(shard);
    assert!(signed.signature().is_empty());
}

#[test]
fn encrypted_carries_key() {
    let shard = make_blob_shard(vec![42]);
    let encrypted = PlainKeys.encrypt(shard);
    assert_eq!(encrypted.key(), &PlainKeys);
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
    let actor: Actor<String, Blob> = Actor::new(
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
    let actor: Actor<String, Blob> = Actor::new(
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
// Custom Keys implementation
// ===========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestKeys {
    label: String,
}

impl Keys for TestKeys {
    fn sign<E>(&self, fragment: Fragment<E>) -> Signed<Self, Fragment<E>> {
        Signed::new(fragment, b"test-sig".to_vec(), self.clone())
    }

    fn encrypt<E: Encode>(&self, fragment: Fragment<E>) -> fragmentation::keys::Encrypted<Self> {
        fragmentation::keys::Encrypted::new(fragment.data().encode(), self.clone())
    }

    fn decrypt<E: Decode>(&self, encrypted: &fragmentation::keys::Encrypted<Self>) -> Fragment<E> {
        let data = E::decode(encrypted.ciphertext()).expect("test decrypt");
        let sha_str = fragment::blob_oid_bytes(encrypted.ciphertext());
        let ref_ = Ref::new(sha::Sha(sha_str), "decrypted");
        Fragment::shard_typed(ref_, data)
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
    let signed = actor.sign(shard);
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
