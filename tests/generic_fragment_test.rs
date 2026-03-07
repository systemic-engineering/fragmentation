use fragmentation::diff;
use fragmentation::encoding::{Decode, Encode};
use fragmentation::fragment::{self, Fragment};
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::store::Store;
use fragmentation::walk;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_bytes_shard(data: Vec<u8>) -> Fragment<Vec<u8>> {
    let r = Ref::new(sha::Sha(fragment::blob_oid_bytes(&data)), "self");
    Fragment::shard_typed(r, data)
}

fn make_bytes_fractal(label: &[u8], children: Vec<Fragment<Vec<u8>>>) -> Fragment<Vec<u8>> {
    let r = Ref::new(sha::Sha(fragment::tree_oid_bytes(label, &children)), "self");
    Fragment::fractal_typed(r, label.to_vec(), children)
}

// ===========================================================================
// Encode/Decode traits
// ===========================================================================

#[test]
fn encode_vec_u8() {
    let data: Vec<u8> = vec![1, 2, 3];
    let encoded = data.encode();
    assert_eq!(encoded, vec![1, 2, 3]);
}

#[test]
fn decode_vec_u8() {
    let bytes = vec![4, 5, 6];
    let decoded = Vec::<u8>::decode(&bytes).unwrap();
    assert_eq!(decoded, vec![4, 5, 6]);
}

#[test]
fn encode_string() {
    let s = "hello".to_string();
    let encoded = s.encode();
    assert_eq!(encoded, b"hello");
}

#[test]
fn decode_string() {
    let bytes = b"hello";
    let decoded = String::decode(bytes).unwrap();
    assert_eq!(decoded, "hello");
}

#[test]
fn decode_string_invalid_utf8() {
    let bytes = vec![0xff, 0xfe];
    assert!(String::decode(&bytes).is_err());
}

// ===========================================================================
// Fragment<Vec<u8>> construction
// ===========================================================================

#[test]
fn bytes_shard_construction() {
    let data = vec![0xCA, 0xFE];
    let shard = make_bytes_shard(data.clone());
    assert!(shard.is_shard());
    assert_eq!(shard.data(), &data);
}

#[test]
fn bytes_fractal_construction() {
    let child = make_bytes_shard(vec![0x01]);
    let parent = make_bytes_fractal(b"root", vec![child.clone()]);
    assert!(parent.is_fractal());
    assert_eq!(parent.data(), &b"root".to_vec());
    assert_eq!(parent.children(), &[child]);
}

#[test]
fn bytes_shard_is_shard() {
    assert!(make_bytes_shard(vec![1]).is_shard());
    assert!(!make_bytes_shard(vec![1]).is_fractal());
}

#[test]
fn bytes_fractal_is_fractal() {
    assert!(make_bytes_fractal(b"x", vec![]).is_fractal());
    assert!(!make_bytes_fractal(b"x", vec![]).is_shard());
}

#[test]
fn bytes_shard_no_children() {
    assert!(make_bytes_shard(vec![1]).children().is_empty());
}

#[test]
fn bytes_fractal_multiple_children() {
    let a = make_bytes_shard(vec![1]);
    let b = make_bytes_shard(vec![2]);
    let parent = make_bytes_fractal(b"parent", vec![a.clone(), b.clone()]);
    assert_eq!(parent.children(), &[a, b]);
}

// ===========================================================================
// content_oid with Fragment<Vec<u8>>
// ===========================================================================

#[test]
fn bytes_content_oid_deterministic() {
    let s = make_bytes_shard(vec![0xDE, 0xAD]);
    let h1 = fragment::content_oid(&s);
    let h2 = fragment::content_oid(&s);
    assert_eq!(h1, h2);
}

#[test]
fn bytes_content_oid_different_data() {
    let s1 = make_bytes_shard(vec![0x01]);
    let s2 = make_bytes_shard(vec![0x02]);
    assert_ne!(fragment::content_oid(&s1), fragment::content_oid(&s2));
}

#[test]
fn bytes_content_oid_is_40_hex_chars() {
    let s = make_bytes_shard(vec![0xFF]);
    let oid = fragment::content_oid(&s);
    assert_eq!(oid.len(), 40);
    assert!(oid.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn bytes_content_oid_shard_vs_fractal() {
    let shard = make_bytes_shard(vec![0x61]); // 'a'
    let frac = make_bytes_fractal(&[0x61], vec![]);
    assert_ne!(fragment::content_oid(&shard), fragment::content_oid(&frac));
}

// ===========================================================================
// Walk with Fragment<Vec<u8>>
// ===========================================================================

#[test]
fn walk_bytes_single_shard() {
    let s = make_bytes_shard(vec![1]);
    let result = walk::collect(&s);
    assert_eq!(result.len(), 1);
}

#[test]
fn walk_bytes_depth_first() {
    let leaf = make_bytes_shard(vec![1]);
    let parent = make_bytes_fractal(b"parent", vec![leaf]);
    let collected = walk::collect(&parent);
    assert_eq!(collected.len(), 2);
}

#[test]
fn walk_bytes_depth() {
    let leaf = make_bytes_shard(vec![1]);
    let mid = make_bytes_fractal(b"mid", vec![leaf]);
    let root = make_bytes_fractal(b"root", vec![mid]);
    assert_eq!(walk::depth(&root), 2);
}

#[test]
fn walk_bytes_fold_count() {
    let root = make_bytes_fractal(
        b"root",
        vec![make_bytes_shard(vec![1]), make_bytes_shard(vec![2])],
    );
    let count = walk::fold(&root, 0, &|acc, _frag| walk::Visitor::Continue(acc + 1));
    assert_eq!(count, 3);
}

#[test]
fn walk_bytes_find() {
    let target = make_bytes_shard(vec![0x42]);
    let other = make_bytes_shard(vec![0x00]);
    let root = make_bytes_fractal(b"root", vec![other, target.clone()]);
    let result = walk::find(&root, &|f| *f.data() == vec![0x42u8]);
    assert_eq!(result, Some(&target));
}

// ===========================================================================
// Store with Fragment<Vec<u8>>
// ===========================================================================

#[test]
fn store_bytes_put_and_get() {
    let frag = make_bytes_shard(vec![0xBE, 0xEF]);
    let mut store: Store<Vec<u8>> = Store::new();
    store.put(frag.clone());
    let sha = &frag.self_ref().sha;
    assert_eq!(store.get(sha), Some(&frag));
}

#[test]
fn store_bytes_dedup() {
    let frag = make_bytes_shard(vec![0xBE, 0xEF]);
    let mut store: Store<Vec<u8>> = Store::new();
    store.put(frag.clone());
    store.put(frag);
    assert_eq!(store.size(), 1);
}

// ===========================================================================
// Diff with Fragment<Vec<u8>>
// ===========================================================================

#[test]
fn diff_bytes_identical() {
    let s = make_bytes_shard(vec![1, 2, 3]);
    let changes = diff::diff(&s, &s);
    assert_eq!(changes, vec![diff::Change::Unchanged(s)]);
}

#[test]
fn diff_bytes_modified() {
    let old = make_bytes_shard(vec![1]);
    let new = make_bytes_shard(vec![2]);
    let changes = diff::diff(&old, &new);
    assert!(changes
        .iter()
        .any(|c| matches!(c, diff::Change::Modified { .. })));
}

#[test]
fn diff_bytes_added_child() {
    let child = make_bytes_shard(vec![1]);
    let old = make_bytes_fractal(b"root", vec![]);
    let new = make_bytes_fractal(b"root", vec![child]);
    let changes = diff::diff(&old, &new);
    assert!(changes.iter().any(|c| matches!(c, diff::Change::Added(_))));
}

// ===========================================================================
// String default still works (existing API unchanged)
// ===========================================================================

#[test]
fn default_type_parameter_is_string() {
    // Fragment without type parameter should be Fragment<String>
    let r = Ref::new(sha::Sha(fragment::blob_oid("hello")), "self");
    let s: Fragment = Fragment::shard(r, "hello");
    assert_eq!(s.data(), "hello");
}
