/// Store: content-addressed fragment storage.
///
/// Maps hash -> Fragment(data). Key is the content hash, computed by encoder.
/// The store is the possibility space made concrete.
import fragmentation.{type Fragment, type Sha}
import gleam/dict.{type Dict}
import gleam/list

// ---------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------

/// Content-addressed fragment store, parameterized by data type.
pub opaque type Store(data) {
  Store(objects: Dict(String, Fragment(data)))
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Create an empty store.
pub fn new() -> Store(data) {
  Store(objects: dict.new())
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// Insert a fragment by its content hash (computed via encoder).
pub fn put(
  store: Store(data),
  frag: Fragment(data),
  encode: fn(data) -> String,
) -> Store(data) {
  let key = fragmentation.hash_fragment(frag, encode)
  Store(objects: dict.insert(store.objects, key, frag))
}

/// Look up a fragment by SHA.
pub fn get(store: Store(data), s: Sha) -> Result(Fragment(data), Nil) {
  let fragmentation.Sha(key) = s
  dict.get(store.objects, key)
}

/// Check if a fragment exists.
pub fn has(store: Store(data), s: Sha) -> Bool {
  let fragmentation.Sha(key) = s
  dict.has_key(store.objects, key)
}

/// Count fragments in the store.
pub fn size(store: Store(data)) -> Int {
  dict.size(store.objects)
}

/// Merge two stores. Same hash = same content, no duplicates.
pub fn merge(a: Store(data), b: Store(data)) -> Store(data) {
  Store(objects: dict.merge(a.objects, b.objects))
}

/// List all SHAs in the store.
pub fn keys(store: Store(data)) -> List(Sha) {
  dict.keys(store.objects)
  |> list.map(fn(k) { fragmentation.sha(k) })
}
