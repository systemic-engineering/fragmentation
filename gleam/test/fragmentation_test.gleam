import fragmentation
import fragmentation/diff
import fragmentation/store
import fragmentation/walk
import gleam/list
import gleam/string
import gleeunit

pub fn main() -> Nil {
  gleeunit.main()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_shard(data: String) -> fragmentation.Fragment(String) {
  let r = fragmentation.ref_(fragmentation.hash(data), "self")
  fragmentation.shard(r, data)
}

fn make_fractal(
  label: String,
  children: List(fragmentation.Fragment(String)),
) -> fragmentation.Fragment(String) {
  let r = fragmentation.ref_(fragmentation.hash(label), "self")
  fragmentation.fractal(r, label, children)
}

// ===========================================================================
// SHA
// ===========================================================================

pub fn sha_construction_test() {
  let s = fragmentation.sha("abc123")
  assert s == fragmentation.Sha("abc123")
}

pub fn hash_returns_sha_test() {
  let s = fragmentation.hash("test")
  let fragmentation.Sha(value) = s
  assert string.length(value) == 128
}

pub fn hash_deterministic_test() {
  let h1 = fragmentation.hash("same")
  let h2 = fragmentation.hash("same")
  assert h1 == h2
}

pub fn hash_different_input_different_sha_test() {
  let h1 = fragmentation.hash("hello")
  let h2 = fragmentation.hash("world")
  assert h1 != h2
}

// ===========================================================================
// Ref
// ===========================================================================

pub fn ref_construction_test() {
  let s = fragmentation.sha("abc")
  let r = fragmentation.ref_(s, "parent")
  assert r == fragmentation.Ref(fragmentation.Sha("abc"), "parent")
}

// ===========================================================================
// Fragment construction
// ===========================================================================

pub fn shard_construction_test() {
  let r = fragmentation.ref_(fragmentation.hash("data"), "self")
  let s = fragmentation.shard(r, "hello")
  assert s == fragmentation.Shard(r, "hello")
}

pub fn fractal_construction_test() {
  let leaf = make_shard("leaf-data")
  let r = fragmentation.ref_(fragmentation.hash("root"), "self")
  let f = fragmentation.fractal(r, "root-data", [leaf])
  assert f == fragmentation.Fractal(r, "root-data", [leaf])
}

pub fn fractal_empty_children_test() {
  let f = make_fractal("empty", [])
  assert fragmentation.children(f) == []
}

pub fn fractal_multiple_children_test() {
  let a = make_shard("alpha")
  let b = make_shard("beta")
  let f = make_fractal("parent", [a, b])
  assert fragmentation.children(f) == [a, b]
}

// ===========================================================================
// Queries
// ===========================================================================

pub fn self_ref_shard_test() {
  let s = make_shard("data")
  let r = fragmentation.self_ref(s)
  let fragmentation.Ref(sha, _) = r
  assert sha == fragmentation.hash("data")
}

pub fn self_ref_fractal_test() {
  let f = make_fractal("node", [])
  let r = fragmentation.self_ref(f)
  let fragmentation.Ref(sha, _) = r
  assert sha == fragmentation.hash("node")
}

pub fn data_shard_test() {
  let s = make_shard("payload")
  assert fragmentation.data(s) == "payload"
}

pub fn data_fractal_test() {
  let f = make_fractal("payload", [])
  assert fragmentation.data(f) == "payload"
}

pub fn is_shard_test() {
  assert fragmentation.is_shard(make_shard("x")) == True
  assert fragmentation.is_shard(make_fractal("x", [])) == False
}

pub fn is_fractal_test() {
  assert fragmentation.is_fractal(make_fractal("x", [])) == True
  assert fragmentation.is_fractal(make_shard("x")) == False
}

pub fn children_shard_test() {
  assert fragmentation.children(make_shard("x")) == []
}

// ===========================================================================
// Content addressing
// ===========================================================================

pub fn hash_fragment_deterministic_test() {
  let encode = fn(x: String) { x }
  let s = make_shard("hello")
  let h1 = fragmentation.hash_fragment(s, encode)
  let h2 = fragmentation.hash_fragment(s, encode)
  assert h1 == h2
}

pub fn hash_fragment_different_data_test() {
  let encode = fn(x: String) { x }
  let s1 = make_shard("hello")
  let s2 = make_shard("world")
  assert fragmentation.hash_fragment(s1, encode) != fragmentation.hash_fragment(s2, encode)
}

pub fn hash_string_fragment_test() {
  let s = make_shard("hello")
  let h = fragmentation.hash_string_fragment(s)
  assert string.length(h) == 128
}

pub fn serialize_roundtrip_hash_test() {
  let encode = fn(x: String) { x }
  let s = make_shard("test")
  let hash_direct = fragmentation.hash_fragment(s, encode)
  let fragmentation.Sha(hash_via_serial) =
    fragmentation.hash(fragmentation.serialize(s, encode))
  assert hash_direct == hash_via_serial
}

pub fn serialize_shard_not_empty_test() {
  let encode = fn(x: String) { x }
  let s = make_shard("data")
  assert fragmentation.serialize(s, encode) != ""
}

pub fn serialize_fractal_not_empty_test() {
  let encode = fn(x: String) { x }
  let f = make_fractal("root", [make_shard("leaf")])
  assert fragmentation.serialize(f, encode) != ""
}

// ===========================================================================
// Store
// ===========================================================================

pub fn store_new_is_empty_test() {
  let s = store.new()
  assert store.size(s) == 0
}

pub fn store_put_and_get_test() {
  let encode = fn(x: String) { x }
  let frag = make_shard("hello")
  let s = store.put(store.new(), frag, encode)
  let key = fragmentation.sha(fragmentation.hash_fragment(frag, encode))
  assert store.get(s, key) == Ok(frag)
}

pub fn store_has_test() {
  let encode = fn(x: String) { x }
  let frag = make_shard("exists")
  let s = store.put(store.new(), frag, encode)
  let key = fragmentation.sha(fragmentation.hash_fragment(frag, encode))
  assert store.has(s, key) == True
  assert store.has(s, fragmentation.sha("nonexistent")) == False
}

pub fn store_size_test() {
  let encode = fn(x: String) { x }
  let s = store.new()
  assert store.size(s) == 0
  let s = store.put(s, make_shard("a"), encode)
  assert store.size(s) == 1
  let s = store.put(s, make_shard("b"), encode)
  assert store.size(s) == 2
}

pub fn store_put_idempotent_test() {
  let encode = fn(x: String) { x }
  let frag = make_shard("same")
  let s = store.put(store.new(), frag, encode)
  let s = store.put(s, frag, encode)
  assert store.size(s) == 1
}

pub fn store_get_missing_test() {
  let s = store.new()
  assert store.get(s, fragmentation.sha("nope")) == Error(Nil)
}

pub fn store_merge_test() {
  let encode = fn(x: String) { x }
  let a = store.put(store.new(), make_shard("alpha"), encode)
  let b = store.put(store.new(), make_shard("beta"), encode)
  let merged = store.merge(a, b)
  assert store.size(merged) == 2
}

pub fn store_merge_dedup_test() {
  let encode = fn(x: String) { x }
  let frag = make_shard("shared")
  let a = store.put(store.new(), frag, encode)
  let b = store.put(store.new(), frag, encode)
  let merged = store.merge(a, b)
  assert store.size(merged) == 1
}

// ===========================================================================
// Walk
// ===========================================================================

pub fn walk_single_shard_test() {
  let s = make_shard("leaf")
  let result = walk.collect(s)
  assert result == [s]
}

pub fn walk_depth_first_test() {
  let leaf = make_shard("leaf")
  let parent = make_fractal("parent", [leaf])
  let collected = walk.collect(parent)
  assert list.length(collected) == 2
  let assert Ok(first) = list.first(collected)
  assert first == parent
}

pub fn walk_nested_three_levels_test() {
  let leaf = make_shard("leaf")
  let mid = make_fractal("mid", [leaf])
  let root = make_fractal("root", [mid])
  let collected = walk.collect(root)
  assert list.length(collected) == 3
}

pub fn walk_wide_tree_test() {
  let a = make_shard("a")
  let b = make_shard("b")
  let c = make_shard("c")
  let root = make_fractal("root", [a, b, c])
  let collected = walk.collect(root)
  assert list.length(collected) == 4
}

pub fn walk_fold_count_test() {
  let root = make_fractal("root", [make_shard("a"), make_shard("b")])
  let count = walk.fold(root, 0, fn(acc, _frag) { walk.Continue(acc + 1) })
  assert count == 3
}

pub fn walk_fold_stop_test() {
  let root = make_fractal("root", [make_shard("a"), make_shard("b")])
  let count = walk.fold(root, 0, fn(acc, _frag) { walk.Stop(acc + 1) })
  assert count == 1
}

pub fn walk_fold_collect_data_test() {
  let root = make_fractal("root", [make_shard("a"), make_shard("b")])
  let data_list =
    walk.fold(root, [], fn(acc, frag) {
      walk.Continue([fragmentation.data(frag), ..acc])
    })
  assert list.length(data_list) == 3
  assert list.contains(data_list, "a")
  assert list.contains(data_list, "b")
  assert list.contains(data_list, "root")
}

pub fn walk_depth_shard_test() {
  assert walk.depth(make_shard("x")) == 0
}

pub fn walk_depth_one_level_test() {
  let parent = make_fractal("parent", [make_shard("leaf")])
  assert walk.depth(parent) == 1
}

pub fn walk_depth_two_levels_test() {
  let leaf = make_shard("leaf")
  let mid = make_fractal("mid", [leaf])
  let root = make_fractal("root", [mid])
  assert walk.depth(root) == 2
}

pub fn walk_depth_asymmetric_test() {
  let deep = make_fractal("deep", [make_shard("leaf")])
  let shallow = make_shard("shallow")
  let root = make_fractal("root", [deep, shallow])
  assert walk.depth(root) == 2
}

pub fn walk_find_test() {
  let target = make_shard("needle")
  let other = make_shard("hay")
  let root = make_fractal("root", [other, target])
  let result = walk.find(root, fn(f) { fragmentation.data(f) == "needle" })
  assert result == Ok(target)
}

pub fn walk_find_not_found_test() {
  let s = make_shard("x")
  let result = walk.find(s, fn(f) { fragmentation.data(f) == "missing" })
  assert result == Error(Nil)
}

pub fn walk_find_nested_test() {
  let target = make_shard("deep-needle")
  let mid = make_fractal("mid", [target])
  let root = make_fractal("root", [make_shard("hay"), mid])
  let result = walk.find(root, fn(f) { fragmentation.data(f) == "deep-needle" })
  assert result == Ok(target)
}

// ===========================================================================
// Diff
// ===========================================================================

pub fn diff_identical_test() {
  let encode = fn(x: String) { x }
  let frag = make_shard("same")
  let changes = diff.diff(frag, frag, encode)
  assert changes == [diff.Unchanged(frag)]
}

pub fn diff_different_roots_test() {
  let encode = fn(x: String) { x }
  let old = make_shard("old")
  let new = make_shard("new")
  let changes = diff.diff(old, new, encode)
  let has_modified =
    list.any(changes, fn(c) {
      case c {
        diff.Modified(_, _) -> True
        _ -> False
      }
    })
  assert has_modified == True
}

pub fn diff_added_child_test() {
  let encode = fn(x: String) { x }
  let child = make_shard("child")
  let old = make_fractal("root", [])
  let new = make_fractal("root", [child])
  let changes = diff.diff(old, new, encode)
  let has_added =
    list.any(changes, fn(c) {
      case c {
        diff.Added(_) -> True
        _ -> False
      }
    })
  assert has_added == True
}

pub fn diff_removed_child_test() {
  let encode = fn(x: String) { x }
  let child = make_shard("child")
  let old = make_fractal("root", [child])
  let new = make_fractal("root", [])
  let changes = diff.diff(old, new, encode)
  let has_removed =
    list.any(changes, fn(c) {
      case c {
        diff.Removed(_) -> True
        _ -> False
      }
    })
  assert has_removed == True
}

pub fn diff_summary_test() {
  let changes = [
    diff.Added(make_shard("x")),
    diff.Removed(make_shard("y")),
    diff.Modified(make_shard("old"), make_shard("new")),
    diff.Unchanged(make_shard("z")),
    diff.Unchanged(make_shard("w")),
  ]
  assert diff.summary(changes) == #(1, 1, 1, 2)
}

pub fn diff_summary_empty_test() {
  assert diff.summary([]) == #(0, 0, 0, 0)
}

// ===========================================================================
// Structural patterns (witness-free)
// ===========================================================================

pub fn parallel_branch_pattern_test() {
  let decision = make_shard("decision:allow")
  let bias_root = make_fractal("bias", [decision])
  let trace = make_fractal("trace", [bias_root])
  let collected = walk.collect(trace)
  assert list.length(collected) == 3
}

pub fn trace_chain_test() {
  let bias = make_shard("bias:v1")
  let t1 = make_fractal("step:observe", [bias])
  let t2 = make_fractal("step:decide", [t1])
  let t3 = make_fractal("step:act", [t2])

  assert walk.depth(t3) == 3
  let collected = walk.collect(t3)
  assert list.length(collected) == 4
}
