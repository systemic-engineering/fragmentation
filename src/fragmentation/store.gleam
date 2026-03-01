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
pub fn put(store: Store, fragment: Fragment) -> Store {
  todo
}

/// Look up a fragment by SHA.
pub fn get(store: Store, sha: Sha) -> Result(Fragment, Nil) {
  todo
}

/// Check if a fragment exists.
pub fn has(store: Store, sha: Sha) -> Bool {
  todo
}

/// Count fragments in the store.
pub fn size(store: Store) -> Int {
  todo
}

/// Merge two stores.
pub fn merge(a: Store, b: Store) -> Store {
  todo
}

/// List all SHAs in the store.
pub fn keys(store: Store) -> List(Sha) {
  todo
}
