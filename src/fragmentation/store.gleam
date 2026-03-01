/// Store: content-addressed fragment storage.
///
/// Maps Sha -> Fragment. Fragments know their own address.
/// The store is the possibility space made concrete.

import fragmentation.{type Fragment, type Sha}
import gleam/dict.{type Dict}

// ---------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------

/// Content-addressed fragment store.
pub opaque type Store {
  Store(fragments: Dict(String, Fragment))
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Create an empty store.
pub fn new() -> Store {
  Store(fragments: dict.new())
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// Insert a fragment by its self-ref SHA.
pub fn put(store: Store, frag: Fragment) -> Store {
  let fragmentation.Ref(fragmentation.Sha(key), _) = fragmentation.self_ref(frag)
  Store(fragments: dict.insert(store.fragments, key, frag))
}

/// Look up a fragment by SHA.
pub fn get(store: Store, s: Sha) -> Result(Fragment, Nil) {
  let fragmentation.Sha(key) = s
  dict.get(store.fragments, key)
}

/// Check if a fragment exists.
pub fn has(store: Store, s: Sha) -> Bool {
  let fragmentation.Sha(key) = s
  dict.has_key(store.fragments, key)
}

/// Count fragments in the store.
pub fn size(store: Store) -> Int {
  dict.size(store.fragments)
}

/// Merge two stores. Same SHA = same content.
pub fn merge(a: Store, b: Store) -> Store {
  Store(fragments: dict.merge(a.fragments, b.fragments))
}

/// List all SHAs in the store.
pub fn keys(store: Store) -> List(Sha) {
  dict.keys(store.fragments)
  |> list.map(fn(k) { fragmentation.Sha(k) })
}

// ---------------------------------------------------------------------------
// Imports
// ---------------------------------------------------------------------------

import gleam/list
