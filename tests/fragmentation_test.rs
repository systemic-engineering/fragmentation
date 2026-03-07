use fragmentation::diff::{self, Change};
use fragmentation::fragment::{self, Fragment};
use fragmentation::ref_::Ref;
use fragmentation::sha;
use fragmentation::store::Store;
use fragmentation::walk;
use fragmentation::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_witnessed() -> Witnessed {
    Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01T00:00:00Z".into()),
        Message("test".into()),
    )
}

fn make_shard(data: &str) -> Fragment {
    let r = Ref::new(sha::hash(data), "self");
    Fragment::shard(r, test_witnessed(), data)
}

fn make_fragment(label: &str, children: Vec<Fragment>) -> Fragment {
    let r = Ref::new(sha::hash(label), "self");
    Fragment::new_fragment(r, test_witnessed(), label, children)
}

// ===========================================================================
// SHA
// ===========================================================================

#[test]
fn sha_construction() {
    let s = sha::Sha("abc123".into());
    assert_eq!(s.0, "abc123");
}

#[test]
fn hash_returns_sha() {
    let s = sha::hash("test");
    assert_eq!(s.0.len(), 64);
}

#[test]
fn hash_deterministic() {
    let h1 = sha::hash("same");
    let h2 = sha::hash("same");
    assert_eq!(h1, h2);
}

#[test]
fn hash_different_input_different_sha() {
    let h1 = sha::hash("hello");
    let h2 = sha::hash("world");
    assert_ne!(h1, h2);
}

// Cross-verification: known SHA-256 values from Gleam
#[test]
fn hash_cross_verify_test() {
    assert_eq!(
        sha::hash("test").0,
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    );
}

#[test]
fn hash_cross_verify_empty() {
    assert_eq!(
        sha::hash("").0,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

// ===========================================================================
// Ref
// ===========================================================================

#[test]
fn ref_construction() {
    let s = sha::Sha("abc".into());
    let r = Ref::new(s.clone(), "parent");
    assert_eq!(r.sha, sha::Sha("abc".into()));
    assert_eq!(r.label, "parent");
}

// Cross-verification: ref serialization
#[test]
fn ref_serialize_cross_verify() {
    let r = Ref::new(sha::Sha("abc123".into()), "self");
    assert_eq!(fragmentation::ref_::serialize_ref(&r), "ref:abc123:self");
}

// ===========================================================================
// Witnessed value types
// ===========================================================================

#[test]
fn author_construction() {
    let a = Author("alex".into());
    assert_eq!(a.0, "alex");
}

#[test]
fn committer_construction() {
    let c = Committer("reed".into());
    assert_eq!(c.0, "reed");
}

#[test]
fn timestamp_construction() {
    let t = Timestamp("2026-03-01T00:00:00Z".into());
    assert_eq!(t.0, "2026-03-01T00:00:00Z");
}

#[test]
fn message_construction() {
    let m = Message("commit msg".into());
    assert_eq!(m.0, "commit msg");
}

// ===========================================================================
// Witnessed
// ===========================================================================

#[test]
fn witnessed_construction() {
    let w = Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01T00:00:00Z".into()),
        Message("initial".into()),
    );
    assert_eq!(w.author, Author("alex".into()));
    assert_eq!(w.committer, Committer("reed".into()));
    assert_eq!(w.timestamp, Timestamp("2026-03-01T00:00:00Z".into()));
    assert_eq!(w.message, Message("initial".into()));
}

#[test]
fn witnessed_serialize_deterministic() {
    let w = test_witnessed();
    let s1 = fragmentation::witnessed::serialize_witnessed(&w);
    let s2 = fragmentation::witnessed::serialize_witnessed(&w);
    assert_eq!(s1, s2);
}

#[test]
fn witnessed_fields_in_serialization() {
    let w = Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01".into()),
        Message("commit msg".into()),
    );
    let s = fragmentation::witnessed::serialize_witnessed(&w);
    assert!(s.contains("author:alex"));
    assert!(s.contains("committer:reed"));
    assert!(s.contains("timestamp:2026-03-01"));
    assert!(s.contains("message:commit msg"));
}

// Cross-verification: witnessed serialization byte-identical to Gleam
#[test]
fn witnessed_serialize_cross_verify() {
    let w = test_witnessed();
    let s = fragmentation::witnessed::serialize_witnessed(&w);
    assert_eq!(
        s,
        "author:alex\ncommitter:reed\ntimestamp:2026-03-01T00:00:00Z\nmessage:test"
    );
}

// ===========================================================================
// Fragment construction
// ===========================================================================

#[test]
fn shard_construction() {
    let r = Ref::new(sha::hash("data"), "self");
    let w = test_witnessed();
    let s = Fragment::shard(r.clone(), w.clone(), "hello");
    assert!(s.is_shard());
    assert_eq!(s.data(), "hello");
    assert_eq!(s.self_ref(), &r);
}

#[test]
fn fragment_construction() {
    let leaf = make_shard("leaf-data");
    let r = Ref::new(sha::hash("root"), "self");
    let w = test_witnessed();
    let f = Fragment::new_fragment(r.clone(), w, "root-data", vec![leaf.clone()]);
    assert!(f.is_fragment());
    assert_eq!(f.data(), "root-data");
    assert_eq!(f.children(), &[leaf]);
}

#[test]
fn fragment_empty_children() {
    let f = make_fragment("empty", vec![]);
    assert!(f.children().is_empty());
}

#[test]
fn fragment_multiple_children() {
    let a = make_shard("alpha");
    let b = make_shard("beta");
    let f = make_fragment("parent", vec![a.clone(), b.clone()]);
    assert_eq!(f.children(), &[a, b]);
}

// ===========================================================================
// Queries
// ===========================================================================

#[test]
fn self_ref_shard() {
    let s = make_shard("data");
    let r = s.self_ref();
    assert_eq!(r.sha, sha::hash("data"));
}

#[test]
fn self_ref_fragment() {
    let f = make_fragment("node", vec![]);
    let r = f.self_ref();
    assert_eq!(r.sha, sha::hash("node"));
}

#[test]
fn self_witnessed() {
    let s = make_shard("x");
    assert_eq!(s.self_witnessed(), &test_witnessed());
}

#[test]
fn data_shard() {
    let s = make_shard("payload");
    assert_eq!(s.data(), "payload");
}

#[test]
fn data_fragment() {
    let f = make_fragment("payload", vec![]);
    assert_eq!(f.data(), "payload");
}

#[test]
fn is_shard_test() {
    assert!(make_shard("x").is_shard());
    assert!(!make_fragment("x", vec![]).is_shard());
}

#[test]
fn is_fragment_test() {
    assert!(make_fragment("x", vec![]).is_fragment());
    assert!(!make_shard("x").is_fragment());
}

#[test]
fn children_shard() {
    assert!(make_shard("x").children().is_empty());
}

// ===========================================================================
// Content addressing
// ===========================================================================

#[test]
fn hash_fragment_deterministic() {
    let s = make_shard("hello");
    let h1 = fragment::hash_fragment(&s);
    let h2 = fragment::hash_fragment(&s);
    assert_eq!(h1, h2);
}

#[test]
fn hash_fragment_different_data() {
    let s1 = make_shard("hello");
    let s2 = make_shard("world");
    assert_ne!(fragment::hash_fragment(&s1), fragment::hash_fragment(&s2));
}

#[test]
fn hash_fragment_witnessed_matters() {
    let r = Ref::new(sha::hash("x"), "self");
    let w1 = Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01".into()),
        Message("first".into()),
    );
    let w2 = Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01".into()),
        Message("second".into()),
    );
    let s1 = Fragment::shard(r.clone(), w1, "same-data");
    let s2 = Fragment::shard(r, w2, "same-data");
    // Different witness = different hash (different witness = different reality)
    assert_ne!(fragment::hash_fragment(&s1), fragment::hash_fragment(&s2));
}

#[test]
fn serialize_roundtrip_hash() {
    let s = make_shard("test");
    let hash_direct = fragment::hash_fragment(&s);
    let hash_via_serial = sha::hash(&fragment::serialize(&s)).0;
    assert_eq!(hash_direct, hash_via_serial);
}

#[test]
fn serialize_shard_not_empty() {
    let s = make_shard("data");
    assert!(!fragment::serialize(&s).is_empty());
}

#[test]
fn serialize_fragment_not_empty() {
    let f = make_fragment("root", vec![make_shard("leaf")]);
    assert!(!fragment::serialize(&f).is_empty());
}

// Cross-verification: shard serialization + hash byte-identical to Gleam
#[test]
fn shard_serialize_cross_verify() {
    let w = test_witnessed();
    let r = Ref::new(sha::hash("hello"), "self");
    let shard = Fragment::shard(r, w, "hello");
    let serialized = fragment::serialize(&shard);
    assert_eq!(
        serialized,
        "shard\nref:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824:self\nauthor:alex\ncommitter:reed\ntimestamp:2026-03-01T00:00:00Z\nmessage:test\ndata:hello"
    );
    assert_eq!(
        fragment::hash_fragment(&shard),
        "8062ef8eb771914801e151d56008935d47afef6c2bbe5ebfe6361180c50de541"
    );
}

// Cross-verification: fragment serialization + hash byte-identical to Gleam
#[test]
fn fragment_serialize_cross_verify() {
    let w = test_witnessed();
    let child = Fragment::shard(Ref::new(sha::hash("leaf"), "self"), w.clone(), "leaf");
    let parent =
        Fragment::new_fragment(Ref::new(sha::hash("root"), "self"), w, "root", vec![child]);
    assert_eq!(
        fragment::hash_fragment(&parent),
        "6a028972957fa706d9cb5eb37a8c359fd745a34d8ac55486ea2156dc2ffc07b9"
    );
}

// ===========================================================================
// Store
// ===========================================================================

#[test]
fn store_new_is_empty() {
    let s = Store::new();
    assert_eq!(s.size(), 0);
}

#[test]
fn store_put_and_get() {
    let frag = make_shard("hello");
    let mut s = Store::new();
    s.put(frag.clone());
    let sha = &frag.self_ref().sha;
    assert_eq!(s.get(sha), Some(&frag));
}

#[test]
fn store_has() {
    let frag = make_shard("exists");
    let mut s = Store::new();
    s.put(frag.clone());
    let sha = &frag.self_ref().sha;
    assert!(s.has(sha));
    assert!(!s.has(&sha::Sha("nonexistent".into())));
}

#[test]
fn store_size() {
    let mut s = Store::new();
    assert_eq!(s.size(), 0);
    s.put(make_shard("a"));
    assert_eq!(s.size(), 1);
    s.put(make_shard("b"));
    assert_eq!(s.size(), 2);
}

#[test]
fn store_put_idempotent() {
    let frag = make_shard("same");
    let mut s = Store::new();
    s.put(frag.clone());
    s.put(frag);
    assert_eq!(s.size(), 1);
}

#[test]
fn store_get_missing() {
    let s = Store::new();
    assert_eq!(s.get(&sha::Sha("nope".into())), None);
}

#[test]
fn store_merge() {
    let mut a = Store::new();
    a.put(make_shard("alpha"));
    let mut b = Store::new();
    b.put(make_shard("beta"));
    a.merge(b);
    assert_eq!(a.size(), 2);
}

#[test]
fn store_merge_dedup() {
    let frag = make_shard("shared");
    let mut a = Store::new();
    a.put(frag.clone());
    let mut b = Store::new();
    b.put(frag);
    a.merge(b);
    assert_eq!(a.size(), 1);
}

// ===========================================================================
// Walk
// ===========================================================================

#[test]
fn walk_single_shard() {
    let s = make_shard("leaf");
    let result = walk::collect(&s);
    assert_eq!(result.len(), 1);
}

#[test]
fn walk_depth_first() {
    let leaf = make_shard("leaf");
    let parent = make_fragment("parent", vec![leaf]);
    let collected = walk::collect(&parent);
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].data(), "parent");
}

#[test]
fn walk_nested_three_levels() {
    let leaf = make_shard("leaf");
    let mid = make_fragment("mid", vec![leaf]);
    let root = make_fragment("root", vec![mid]);
    let collected = walk::collect(&root);
    assert_eq!(collected.len(), 3);
}

#[test]
fn walk_wide_tree() {
    let a = make_shard("a");
    let b = make_shard("b");
    let c = make_shard("c");
    let root = make_fragment("root", vec![a, b, c]);
    let collected = walk::collect(&root);
    assert_eq!(collected.len(), 4);
}

#[test]
fn walk_fold_count() {
    let root = make_fragment("root", vec![make_shard("a"), make_shard("b")]);
    let count = walk::fold(&root, 0, &|acc, _frag| walk::Visitor::Continue(acc + 1));
    assert_eq!(count, 3);
}

#[test]
fn walk_fold_stop() {
    let root = make_fragment("root", vec![make_shard("a"), make_shard("b")]);
    let count = walk::fold(&root, 0, &|acc, _frag| walk::Visitor::Stop(acc + 1));
    assert_eq!(count, 1);
}

#[test]
fn walk_fold_collect_data() {
    let root = make_fragment("root", vec![make_shard("a"), make_shard("b")]);
    let data = walk::fold(&root, vec![], &|mut acc, frag| {
        acc.push(frag.data().to_string());
        walk::Visitor::Continue(acc)
    });
    assert_eq!(data.len(), 3);
    assert!(data.contains(&"a".to_string()));
    assert!(data.contains(&"b".to_string()));
    assert!(data.contains(&"root".to_string()));
}

#[test]
fn walk_depth_shard() {
    assert_eq!(walk::depth(&make_shard("x")), 0);
}

#[test]
fn walk_depth_one_level() {
    let parent = make_fragment("parent", vec![make_shard("leaf")]);
    assert_eq!(walk::depth(&parent), 1);
}

#[test]
fn walk_depth_two_levels() {
    let leaf = make_shard("leaf");
    let mid = make_fragment("mid", vec![leaf]);
    let root = make_fragment("root", vec![mid]);
    assert_eq!(walk::depth(&root), 2);
}

#[test]
fn walk_depth_asymmetric() {
    let deep = make_fragment("deep", vec![make_shard("leaf")]);
    let shallow = make_shard("shallow");
    let root = make_fragment("root", vec![deep, shallow]);
    assert_eq!(walk::depth(&root), 2);
}

#[test]
fn walk_find() {
    let target = make_shard("needle");
    let other = make_shard("hay");
    let root = make_fragment("root", vec![other, target.clone()]);
    let result = walk::find(&root, &|f| f.data() == "needle");
    assert_eq!(result, Some(&target));
}

#[test]
fn walk_find_not_found() {
    let s = make_shard("x");
    let result = walk::find(&s, &|f| f.data() == "missing");
    assert_eq!(result, None);
}

#[test]
fn walk_find_nested() {
    let target = make_shard("deep-needle");
    let mid = make_fragment("mid", vec![target.clone()]);
    let root = make_fragment("root", vec![make_shard("hay"), mid]);
    let result = walk::find(&root, &|f| f.data() == "deep-needle");
    assert_eq!(result, Some(&target));
}

// ===========================================================================
// Diff
// ===========================================================================

#[test]
fn diff_identical() {
    let s = make_shard("same");
    let changes = diff::diff(&s, &s);
    assert_eq!(changes, vec![Change::Unchanged(s)]);
}

#[test]
fn diff_different_roots() {
    let old = make_shard("old");
    let new = make_shard("new");
    let changes = diff::diff(&old, &new);
    assert!(changes.iter().any(|c| matches!(c, Change::Modified { .. })));
}

#[test]
fn diff_added_child() {
    let child = make_shard("child");
    let old = make_fragment("root", vec![]);
    let new = make_fragment("root", vec![child]);
    let changes = diff::diff(&old, &new);
    assert!(changes.iter().any(|c| matches!(c, Change::Added(_))));
}

#[test]
fn diff_removed_child() {
    let child = make_shard("child");
    let old = make_fragment("root", vec![child]);
    let new = make_fragment("root", vec![]);
    let changes = diff::diff(&old, &new);
    assert!(changes.iter().any(|c| matches!(c, Change::Removed(_))));
}

#[test]
fn diff_summary() {
    let changes = vec![
        Change::Added(make_shard("x")),
        Change::Removed(make_shard("y")),
        Change::Modified {
            old: make_shard("old"),
            new: make_shard("new"),
        },
        Change::Unchanged(make_shard("z")),
        Change::Unchanged(make_shard("w")),
    ];
    assert_eq!(diff::summary(&changes), (1, 1, 1, 2));
}

#[test]
fn diff_summary_empty() {
    assert_eq!(diff::summary(&[]), (0, 0, 0, 0));
}

// ===========================================================================
// Witnessed reality (trace-as-branch)
// ===========================================================================

#[test]
fn different_witness_different_hash() {
    let r = Ref::new(sha::hash("x"), "self");
    let w_alex = Witnessed::new(
        Author("alex".into()),
        Committer("alex".into()),
        Timestamp("2026-03-01".into()),
        Message("observed".into()),
    );
    let w_reed = Witnessed::new(
        Author("reed".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01".into()),
        Message("traced".into()),
    );
    let s_alex = Fragment::shard(r.clone(), w_alex, "same-data");
    let s_reed = Fragment::shard(r, w_reed, "same-data");
    assert_ne!(
        fragment::hash_fragment(&s_alex),
        fragment::hash_fragment(&s_reed)
    );
}

#[test]
fn parallel_branch_pattern() {
    let decision = make_shard("decision:allow");
    let bias_root = make_fragment("bias", vec![decision]);
    let trace = make_fragment("trace", vec![bias_root]);
    let collected = walk::collect(&trace);
    assert_eq!(collected.len(), 3);
}

#[test]
fn trace_chain() {
    let bias = make_shard("bias:v1");
    let t1 = make_fragment("step:observe", vec![bias]);
    let t2 = make_fragment("step:decide", vec![t1]);
    let t3 = make_fragment("step:act", vec![t2]);
    assert_eq!(walk::depth(&t3), 3);
    let collected = walk::collect(&t3);
    assert_eq!(collected.len(), 4);
}

#[test]
fn author_committer_split() {
    let r = Ref::new(sha::hash("decision"), "self");
    let w = Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01T19:30:00Z".into()),
        Message("bias execution trace".into()),
    );
    let traced = Fragment::shard(r, w, "decision:allow");
    let witness = traced.self_witnessed();
    assert_eq!(witness.author, Author("alex".into()));
    assert_eq!(witness.committer, Committer("reed".into()));
    assert_eq!(witness.message, Message("bias execution trace".into()));
}
