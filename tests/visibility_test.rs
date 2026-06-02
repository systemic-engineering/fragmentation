use fragmentation::fragment::{self, ContentAddressed, Fractal, Fragmentable, TreeShaped};
use fragmentation::keys::{Keys, PlainKeys};
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::visibility::{Private, Protected, Public};
use fragmentation::walk;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_shard(data: &str) -> Fractal<String> {
    let r = Ref::new(sha::Sha(fragment::blob_oid(data)), "self");
    Fractal::shard(r, data)
}

fn make_fractal(label: &str, children: Vec<Fractal<String>>) -> Fractal<String> {
    let r = Ref::new(sha::Sha(fragment::tree_oid(label, &children)), "self");
    Fractal::new(r, label, children)
}

/// PlainKeys signature — empty bytes, no proof needed.
/// The Signature wrapper enforces that only Keys::sign can create these.
fn plain_sig() -> fragmentation::keys::Signature<PlainKeys> {
    let dummy = make_shard("_");
    PlainKeys.sign(&dummy).unwrap()
}

// ===========================================================================
// Public
// ===========================================================================

#[test]
fn public_construction() {
    let shard = make_shard("hello");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard.clone(), sig);
    assert_eq!(public.inner(), &shard);
}

#[test]
fn public_inner_access() {
    let shard = make_shard("data");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert_eq!(public.inner().data(), "data");
}

#[test]
fn public_into_inner() {
    let shard = make_shard("unwrap");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard.clone(), sig);
    let recovered = public.into_inner();
    assert_eq!(recovered, shard);
}

#[test]
fn public_key() {
    let shard = make_shard("keyed");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert_eq!(public.key(), &PlainKeys);
}

#[test]
fn public_plain_signature_bytes_empty() {
    // PlainKeys produces empty proof bytes. The Signature wrapper
    // ensures you can only get one through Keys::sign — no faking.
    let shard = make_shard("unsigned");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert!(public.signature().bytes().is_empty());
    assert_eq!(public.signature().key(), &PlainKeys);
}

#[test]
fn public_fragment_self_ref() {
    let shard = make_shard("ref-test");
    let expected_ref = shard.self_ref().clone();
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert_eq!(public.self_ref(), &expected_ref);
}

#[test]
fn public_fragment_data() {
    let shard = make_shard("visible");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert_eq!(public.data(), "visible");
}

#[test]
fn public_fragment_children_empty() {
    let parent = make_fractal("parent", vec![make_shard("child")]);
    let sig = PlainKeys.sign(&parent).unwrap();
    let public = Public::new(parent, sig);
    assert!(public.children().is_empty());
}

#[test]
fn public_fragment_is_shard() {
    let shard = make_shard("terminal");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert!(public.is_shard());
}

#[test]
fn public_fragment_is_fractal_default() {
    // Public<K, T> does not override is_fractal — exercises the trait default (fragment.rs:22-27)
    let shard = make_shard("terminal");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    assert!(!public.is_fractal());
}

#[test]
fn public_walk_terminal() {
    let shard = make_shard("single");
    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard, sig);
    let collected = walk::collect(&public);
    assert_eq!(collected.len(), 1);
}

// ===========================================================================
// Protected
// ===========================================================================

#[test]
fn protected_construction() {
    let r = Ref::new(sha::Sha("abc".into()), "test");
    let protected = Protected::new(r.clone(), vec![1, 2, 3], plain_sig());
    assert_eq!(protected.self_ref(), &r);
    assert_eq!(protected.ciphertext(), &[1, 2, 3]);
}

#[test]
fn protected_ciphertext() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let protected = Protected::new(r, vec![0xCA, 0xFE], plain_sig());
    assert_eq!(protected.ciphertext(), &[0xCA, 0xFE]);
}

#[test]
fn protected_key() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let protected = Protected::new(r, vec![], plain_sig());
    assert_eq!(protected.key(), &PlainKeys);
}

#[test]
fn protected_signature() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let protected = Protected::new(r, vec![], plain_sig());
    assert!(protected.signature().bytes().is_empty());
    assert_eq!(protected.signature().key(), &PlainKeys);
}

#[test]
fn protected_fragment_self_ref() {
    let r = Ref::new(sha::Sha("plaintext-sha".into()), "original");
    let protected = Protected::new(r.clone(), vec![1], plain_sig());
    assert_eq!(protected.self_ref(), &r);
}

#[test]
fn protected_fragment_data_is_ciphertext() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let protected = Protected::new(r, vec![0xDE, 0xAD], plain_sig());
    assert_eq!(protected.data(), &vec![0xDE, 0xAD]);
}

#[test]
fn protected_fragment_children_empty() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let protected = Protected::new(r, vec![], plain_sig());
    assert!(protected.children().is_empty());
}

#[test]
fn protected_wrap_from_fractal() {
    let shard = make_shard("secret");
    let sig = PlainKeys.sign(&shard).unwrap();
    let protected = Protected::wrap(shard, sig).unwrap();
    assert!(!protected.ciphertext().is_empty());
}

#[test]
fn protected_unlock() {
    let shard = make_shard("secret");
    let sig = PlainKeys.sign(&shard).unwrap();
    let protected = Protected::wrap(shard, sig).unwrap();
    let recovered: Fractal<String> = protected.unlock().unwrap();
    assert_eq!(recovered.data(), "secret");
}

#[test]
fn protected_ref_matches_original() {
    let shard = make_shard("integrity");
    let original_ref = shard.self_ref().clone();
    let sig = PlainKeys.sign(&shard).unwrap();
    let protected = Protected::wrap(shard, sig).unwrap();
    assert_eq!(protected.self_ref(), &original_ref);
}

#[test]
fn protected_walk_terminal() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let protected = Protected::new(r, vec![1], plain_sig());
    let collected = walk::collect(&protected);
    assert_eq!(collected.len(), 1);
}

// ===========================================================================
// Private
// ===========================================================================

#[test]
fn private_construction() {
    let r = Ref::new(sha::Sha("proof".into()), "exists");
    let private = Private::new(r.clone(), plain_sig());
    assert_eq!(private.self_ref(), &r);
}

#[test]
fn private_key() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let private = Private::new(r, plain_sig());
    assert_eq!(private.key(), &PlainKeys);
}

#[test]
fn private_signature() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let private = Private::new(r, plain_sig());
    assert!(private.signature().bytes().is_empty());
    assert_eq!(private.signature().key(), &PlainKeys);
}

#[test]
fn private_seal() {
    let shard = make_shard("sealed");
    let sig = PlainKeys.sign(&shard).unwrap();
    let private = Private::seal(&shard, sig);
    assert_eq!(private.self_ref(), shard.self_ref());
}

#[test]
fn private_fragment_self_ref() {
    let r = Ref::new(sha::Sha("hash-only".into()), "proof");
    let private = Private::new(r.clone(), plain_sig());
    assert_eq!(private.self_ref(), &r);
}

#[test]
fn private_fragment_data_is_unit() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let private = Private::new(r, plain_sig());
    assert_eq!(private.data(), &());
}

#[test]
fn private_fragment_children_empty() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let private = Private::new(r, plain_sig());
    assert!(private.children().is_empty());
}

#[test]
fn private_ref_matches_original() {
    let shard = make_shard("original");
    let original_ref = shard.self_ref().clone();
    let sig = PlainKeys.sign(&shard).unwrap();
    let private = Private::seal(&shard, sig);
    assert_eq!(private.self_ref(), &original_ref);
}

#[test]
fn private_walk_terminal() {
    let r = Ref::new(sha::Sha("x".into()), "test");
    let private = Private::new(r, plain_sig());
    let collected = walk::collect(&private);
    assert_eq!(collected.len(), 1);
}

// ===========================================================================
// Cross-visibility: SHA accessible from all three
// ===========================================================================

#[test]
fn sha_accessible_from_all_three() {
    let shard = make_shard("shared");
    let sha = shard.self_ref().sha.clone();

    let sig = PlainKeys.sign(&shard).unwrap();
    let public = Public::new(shard.clone(), sig);

    let sig = PlainKeys.sign(&shard).unwrap();
    let protected = Protected::wrap(shard.clone(), sig).unwrap();

    let sig = PlainKeys.sign(&shard).unwrap();
    let private = Private::seal(&shard, sig);

    assert_eq!(public.self_ref().sha, sha);
    assert_eq!(protected.self_ref().sha, sha);
    assert_eq!(private.self_ref().sha, sha);
}

// ===========================================================================
// Mixed-visibility integration
// ===========================================================================

/// A tree where each child is wrapped in a different visibility level.
/// The SHA from the original fragment is accessible from all three wrappers.
#[test]
fn mixed_visibility_children_preserve_sha() {
    let child_a = make_shard("alpha");
    let child_b = make_shard("beta");
    let child_c = make_shard("gamma");
    let parent = make_fractal(
        "parent",
        vec![child_a.clone(), child_b.clone(), child_c.clone()],
    );

    let sig_a = PlainKeys.sign(&child_a).unwrap();
    let pub_a = Public::new(child_a.clone(), sig_a);

    let sig_b = PlainKeys.sign(&child_b).unwrap();
    let prot_b = Protected::wrap(child_b.clone(), sig_b).unwrap();

    let sig_c = PlainKeys.sign(&child_c).unwrap();
    let priv_c = Private::seal(&child_c, sig_c);

    // All three wrappers expose the original SHA
    assert_eq!(pub_a.self_ref().sha, child_a.self_ref().sha);
    assert_eq!(prot_b.self_ref().sha, child_b.self_ref().sha);
    assert_eq!(priv_c.self_ref().sha, child_c.self_ref().sha);

    // Parent still knows about its children
    assert_eq!(parent.children().len(), 3);
}

/// Merkle property: wrapping children in different visibility levels doesn't
/// change their SHAs, because visibility delegates self_ref to the stored ref.
/// The parent's SHA incorporates child SHAs directly.
#[test]
fn merkle_stability_across_visibility_boundaries() {
    let child_a = make_shard("one");
    let child_b = make_shard("two");
    let child_c = make_shard("three");

    // Capture original child SHAs
    let sha_a = child_a.self_ref().sha.clone();
    let sha_b = child_b.self_ref().sha.clone();
    let sha_c = child_c.self_ref().sha.clone();

    // Build the parent from unwrapped children
    let parent = make_fractal(
        "root",
        vec![child_a.clone(), child_b.clone(), child_c.clone()],
    );
    let parent_sha = parent.self_ref().sha.clone();

    // Wrap each child differently
    let sig_a = PlainKeys.sign(&child_a).unwrap();
    let pub_a = Public::new(child_a, sig_a);

    let sig_b = PlainKeys.sign(&child_b).unwrap();
    let prot_b = Protected::wrap(child_b, sig_b).unwrap();

    let sig_c = PlainKeys.sign(&child_c).unwrap();
    let priv_c = Private::seal(&child_c, sig_c);

    // Child SHAs are unchanged by wrapping
    assert_eq!(pub_a.self_ref().sha, sha_a);
    assert_eq!(prot_b.self_ref().sha, sha_b);
    assert_eq!(priv_c.self_ref().sha, sha_c);

    // Parent SHA is deterministic and stable (rebuilding yields same hash)
    let parent_rebuilt = make_fractal(
        "root",
        vec![make_shard("one"), make_shard("two"), make_shard("three")],
    );
    assert_eq!(parent_rebuilt.self_ref().sha, parent_sha);
}

/// walk::collect on each visibility wrapper returns exactly 1 (terminal),
/// proving the visibility boundary stops traversal even when the inner
/// fragment has children.
#[test]
fn walk_collect_stops_at_visibility_boundary() {
    let deep = make_fractal(
        "deep",
        vec![
            make_shard("leaf-1"),
            make_fractal("mid", vec![make_shard("leaf-2")]),
        ],
    );

    // Unwrapped: 4 nodes (deep -> leaf-1, mid -> leaf-2)
    assert_eq!(walk::collect(&deep).len(), 4);

    // Each visibility wrapper is terminal
    let sig = PlainKeys.sign(&deep).unwrap();
    let pub_deep = Public::new(deep.clone(), sig);
    assert_eq!(walk::collect(&pub_deep).len(), 1);

    let sig = PlainKeys.sign(&deep).unwrap();
    let prot_deep = Protected::wrap(deep.clone(), sig).unwrap();
    assert_eq!(walk::collect(&prot_deep).len(), 1);

    let sig = PlainKeys.sign(&deep).unwrap();
    let priv_deep = Private::seal(&deep, sig);
    assert_eq!(walk::collect(&priv_deep).len(), 1);
}

/// Protected::wrap preserves the original ref on the wrapper.
/// unlock recovers the data content, and the wrapper's self_ref
/// matches the original fragment's ref throughout.
#[test]
fn protected_wrap_unlock_roundtrip_preserves_ref_and_data() {
    let original = make_fractal("parent", vec![make_shard("child-a"), make_shard("child-b")]);
    let original_ref = original.self_ref().clone();
    let original_data = original.data().clone();

    let sig = PlainKeys.sign(&original).unwrap();
    let protected = Protected::wrap(original, sig).unwrap();

    // The protected wrapper preserves the original ref
    assert_eq!(protected.self_ref(), &original_ref);

    // Unlock recovers the data content
    let recovered: Fractal<String> = protected.unlock().unwrap();
    assert_eq!(recovered.data(), &original_data);
}

/// A "mixed bag": refs collected from Public, Protected, Private wrappers
/// all match the original children's refs.
#[test]
fn mixed_bag_refs_match_originals() {
    let children: Vec<Fractal<String>> = vec![
        make_shard("public-data"),
        make_shard("protected-data"),
        make_shard("private-data"),
    ];

    // Collect original refs
    let original_refs: Vec<Ref> = children.iter().map(|c| c.self_ref().clone()).collect();

    // Wrap each child with a different visibility level
    let sig = PlainKeys.sign(&children[0]).unwrap();
    let pub_wrap = Public::new(children[0].clone(), sig);

    let sig = PlainKeys.sign(&children[1]).unwrap();
    let prot_wrap = Protected::wrap(children[1].clone(), sig).unwrap();

    let sig = PlainKeys.sign(&children[2]).unwrap();
    let priv_wrap = Private::seal(&children[2], sig);

    // Collect refs from wrappers
    let wrapped_refs: Vec<&Ref> = vec![
        pub_wrap.self_ref(),
        prot_wrap.self_ref(),
        priv_wrap.self_ref(),
    ];

    // Every wrapped ref matches its original
    for (original, wrapped) in original_refs.iter().zip(wrapped_refs.iter()) {
        assert_eq!(original.sha, wrapped.sha);
    }

    // The refs are all distinct from each other (different content = different SHA)
    assert_ne!(wrapped_refs[0].sha, wrapped_refs[1].sha);
    assert_ne!(wrapped_refs[1].sha, wrapped_refs[2].sha);
    assert_ne!(wrapped_refs[0].sha, wrapped_refs[2].sha);
}

#[test]
fn private_content_oid_exercises_unit_encode() {
    // Private<K> has Data = (), exercising Encode for ()
    let shard = make_shard("secret");
    let sig = PlainKeys.sign(&shard).unwrap();
    let private = Private::seal(&shard, sig);
    let oid = fragment::content_oid(&private);
    assert!(!oid.is_empty());
}
