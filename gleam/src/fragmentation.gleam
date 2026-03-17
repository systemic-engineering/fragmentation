// Fragmentation: encoded possibility space.
//
// Content-addressed, arbitrary depth, circular-reflexive.
// Reality for git.
//
// Every fragment knows its own address (Ref) and holds data.
// Shards are terminal. Fractals continue.
// The observer is part of the commit, not the hash.

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

/// A node in the possibility space.
pub type Fragment(data) {
  /// Terminal: self-addressed, carries data, stops.
  Shard(ref_: Ref, data: data)
  /// Self-similar: self-addressed, carries data, contains fractal children.
  Fractal(ref_: Ref, data: data, children: List(Fragment(data)))
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Create a SHA from a raw string.
pub fn sha(value: String) -> Sha {
  Sha(self: value)
}

/// Create a reference.
pub fn ref_(s: Sha, label: String) -> Ref {
  Ref(sha: s, label: label)
}

/// Create a shard. Terminal fragment.
pub fn shard(ref_: Ref, data: data) -> Fragment(data) {
  Shard(ref_: ref_, data: data)
}

/// Create a fractal. Self-similar, contains other fragments.
pub fn fractal(ref_: Ref, data: data, children: List(Fragment(data))) -> Fragment(data) {
  Fractal(ref_: ref_, data: data, children: children)
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// SHA-512 hash of a string. Returns Sha.
pub fn hash(data: String) -> Sha {
  Sha(self: sha512(data))
}

/// Deterministic canonical serialization of a ref.
fn serialize_ref(r: Ref) -> String {
  let Ref(Sha(s), label) = r
  "ref:" <> s <> ":" <> label
}

/// Deterministic canonical serialization of a fragment, using encoder for data.
pub fn serialize(frag: Fragment(data), encode: fn(data) -> String) -> String {
  case frag {
    Shard(r, d) ->
      "shard\n"
      <> serialize_ref(r)
      <> "\ndata:"
      <> encode(d)
    Fractal(r, d, cs) ->
      "fractal\n"
      <> serialize_ref(r)
      <> "\ndata:"
      <> encode(d)
      <> "\nchildren:["
      <> {
        cs
        |> list.map(fn(f) { serialize(f, encode) })
        |> string.join(",")
      }
      <> "]"
  }
}

/// Content-address a fragment: SHA-512 of its canonical serialization.
pub fn hash_fragment(frag: Fragment(data), encode: fn(data) -> String) -> String {
  sha512(serialize(frag, encode))
}

/// Convenience: content-address a Fragment(String) with identity encoder.
pub fn hash_string_fragment(frag: Fragment(String)) -> String {
  hash_fragment(frag, fn(s) { s })
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get the ref (self-address) of a fragment.
pub fn self_ref(frag: Fragment(data)) -> Ref {
  case frag {
    Shard(r, _) -> r
    Fractal(r, _, _) -> r
  }
}

/// Get the data from a fragment.
pub fn data(frag: Fragment(data)) -> data {
  case frag {
    Shard(_, d) -> d
    Fractal(_, d, _) -> d
  }
}

/// Get child fragments. Shards have none.
pub fn children(frag: Fragment(data)) -> List(Fragment(data)) {
  case frag {
    Shard(_, _) -> []
    Fractal(_, _, cs) -> cs
  }
}

/// Check if a fragment is a shard.
pub fn is_shard(frag: Fragment(data)) -> Bool {
  case frag {
    Shard(_, _) -> True
    Fractal(_, _, _) -> False
  }
}

/// Check if a fragment is a fractal (non-terminal).
pub fn is_fractal(frag: Fragment(data)) -> Bool {
  case frag {
    Shard(_, _) -> False
    Fractal(_, _, _) -> True
  }
}

// ---------------------------------------------------------------------------
// FFI
// ---------------------------------------------------------------------------

@external(erlang, "fragmentation_ffi", "sha512")
fn sha512(data: String) -> String
