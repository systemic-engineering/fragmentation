// Fragmentation: encoded possibility space.
//
// Content-addressed, arbitrary depth, circular-reflexive.
// Reality for git.
//
// A Fragment is a node in the possibility space.
// Every fragment knows its own address (Ref) and carries data.
// Shards are terminal. Fractals continue — self-similar structure
// at every scale, arbitrary width, arbitrary depth.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Content-addressed hash.
pub type Sha {
  Sha(self: String)
}

/// A reference: address + label.
pub type Ref {
  Ref(sha: Sha, label: String)
}

/// A node in the possibility space.
pub type Fragment {
  /// Terminal: self-addressed, carries data, stops.
  Shard(ref: Ref, data: String)
  /// Self-similar: self-addressed, carries data, contains fragments.
  Fractal(ref: Ref, data: String, fragments: List(Fragment))
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Create a SHA from a raw string.
pub fn sha(value: String) -> Sha {
  Sha(self: value)
}

/// Create a reference.
pub fn ref(sha: Sha, label: String) -> Ref {
  Ref(sha: sha, label: label)
}

/// Create a shard. Terminal fragment.
pub fn shard(ref: Ref, data: String) -> Fragment {
  Shard(ref: ref, data: data)
}

/// Create a fractal. Self-similar fragment.
pub fn fractal(
  ref: Ref,
  data: String,
  fragments: List(Fragment),
) -> Fragment {
  Fractal(ref: ref, data: data, fragments: fragments)
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Raw SHA-256 hash of a string.
pub fn hash(data: String) -> Sha {
  Sha(self: sha256(data))
}

/// Deterministic canonical serialization of a fragment.
pub fn serialize(fragment: Fragment) -> String {
  todo
}

/// Content-address a fragment: SHA-256 of its canonical serialization.
pub fn hash_fragment(fragment: Fragment) -> String {
  todo
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get the ref (self-address) of a fragment.
pub fn self_ref(fragment: Fragment) -> Ref {
  case fragment {
    Shard(r, _) -> r
    Fractal(r, _, _) -> r
  }
}

/// Get the data from a fragment.
pub fn data(fragment: Fragment) -> String {
  case fragment {
    Shard(_, d) -> d
    Fractal(_, d, _) -> d
  }
}

/// Get child fragments. Shards have none.
pub fn children(fragment: Fragment) -> List(Fragment) {
  case fragment {
    Shard(_, _) -> []
    Fractal(_, _, fs) -> fs
  }
}

/// Check if a fragment is a shard.
pub fn is_shard(fragment: Fragment) -> Bool {
  case fragment {
    Shard(_, _) -> True
    Fractal(_, _, _) -> False
  }
}

/// Check if a fragment is a fractal.
pub fn is_fractal(fragment: Fragment) -> Bool {
  case fragment {
    Shard(_, _) -> False
    Fractal(_, _, _) -> True
  }
}

// ---------------------------------------------------------------------------
// FFI
// ---------------------------------------------------------------------------

@external(erlang, "fragmentation_ffi", "sha256")
fn sha256(data: String) -> String
