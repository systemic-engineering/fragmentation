// Fragmentation: encoded possibility space.
//
// Content-addressed, arbitrary depth, circular-reflexive.
// Reality for git.
//
// Every fragment knows its own address (Ref), is witnessed (Witnessed),
// and holds data. Shards are terminal. Fragments continue.
// Witnessed is git: author, committer, timestamp, message.

import gleam/list
import gleam/string

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

/// Git commit metadata. Who was here when this happened.
pub type Witnessed {
  Witnessed(
    author: String,
    committer: String,
    timestamp: String,
    message: String,
  )
}

/// A node in the possibility space.
pub type Fragment {
  /// Terminal: self-addressed, witnessed, carries data, stops.
  Shard(ref: Ref, witnessed: Witnessed, data: String)
  /// Self-similar: self-addressed, witnessed, carries data, contains fragments.
  Fragment(ref: Ref, witnessed: Witnessed, data: String, fragments: List(Fragment))
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Create a SHA from a raw string.
pub fn sha(value: String) -> Sha {
  Sha(self: value)
}

/// Create a reference.
pub fn ref(s: Sha, label: String) -> Ref {
  Ref(sha: s, label: label)
}

/// Create a witness record.
pub fn witnessed(
  author: String,
  committer: String,
  timestamp: String,
  message: String,
) -> Witnessed {
  Witnessed(
    author: author,
    committer: committer,
    timestamp: timestamp,
    message: message,
  )
}

/// Create a shard. Terminal fragment.
pub fn shard(ref: Ref, witnessed: Witnessed, data: String) -> Fragment {
  Shard(ref: ref, witnessed: witnessed, data: data)
}

/// Create a fragment. Self-similar, contains other fragments.
pub fn fragment(
  ref: Ref,
  witnessed: Witnessed,
  data: String,
  fragments: List(Fragment),
) -> Fragment {
  Fragment(ref: ref, witnessed: witnessed, data: data, fragments: fragments)
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Raw SHA-256 hash of a string.
pub fn hash(data: String) -> Sha {
  Sha(self: sha256(data))
}

/// Deterministic canonical serialization of a witness record.
pub fn serialize_witnessed(m: Witnessed) -> String {
  "author:"
  <> m.author
  <> "\ncommitter:"
  <> m.committer
  <> "\ntimestamp:"
  <> m.timestamp
  <> "\nmessage:"
  <> m.message
}

/// Deterministic canonical serialization of a ref.
pub fn serialize_ref(r: Ref) -> String {
  let Ref(Sha(s), label) = r
  "ref:" <> s <> ":" <> label
}

/// Deterministic canonical serialization of a fragment.
pub fn serialize(frag: Fragment) -> String {
  case frag {
    Shard(r, m, d) ->
      "shard\n"
      <> serialize_ref(r)
      <> "\n"
      <> serialize_witnessed(m)
      <> "\ndata:"
      <> d
    Fragment(r, m, d, fs) ->
      "fragment\n"
      <> serialize_ref(r)
      <> "\n"
      <> serialize_witnessed(m)
      <> "\ndata:"
      <> d
      <> "\nfragments:["
      <> {
        fs
        |> list.map(fn(f) { serialize(f) })
        |> string.join(",")
      }
      <> "]"
  }
}

/// Content-address a fragment: SHA-256 of its canonical serialization.
pub fn hash_fragment(frag: Fragment) -> String {
  sha256(serialize(frag))
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get the ref (self-address) of a fragment.
pub fn self_ref(frag: Fragment) -> Ref {
  case frag {
    Shard(r, _, _) -> r
    Fragment(r, _, _, _) -> r
  }
}

/// Get the witness record of a fragment.
pub fn self_witnessed(frag: Fragment) -> Witnessed {
  case frag {
    Shard(_, w, _) -> w
    Fragment(_, w, _, _) -> w
  }
}

/// Get the data from a fragment.
pub fn data(frag: Fragment) -> String {
  case frag {
    Shard(_, _, d) -> d
    Fragment(_, _, d, _) -> d
  }
}

/// Get child fragments. Shards have none.
pub fn children(frag: Fragment) -> List(Fragment) {
  case frag {
    Shard(_, _, _) -> []
    Fragment(_, _, _, fs) -> fs
  }
}

/// Check if a fragment is a shard.
pub fn is_shard(frag: Fragment) -> Bool {
  case frag {
    Shard(_, _, _) -> True
    Fragment(_, _, _, _) -> False
  }
}

/// Check if a fragment is a fragment (non-terminal).
pub fn is_fragment(frag: Fragment) -> Bool {
  case frag {
    Shard(_, _, _) -> False
    Fragment(_, _, _, _) -> True
  }
}

// ---------------------------------------------------------------------------
// FFI
// ---------------------------------------------------------------------------

@external(erlang, "fragmentation_ffi", "sha256")
fn sha256(data: String) -> String
