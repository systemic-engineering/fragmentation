/// git: content-addressed fragment persistence.
///
/// Writes fragments to disk named by their content hash.
/// The store is a directory. Each fragment becomes a file.
/// File name = SHA-512 of canonical serialization.
/// Idempotent: same hash, same content, same file.
import fragmentation
import simplifile

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// Write a fragment to disk under `dir`, named by its content-addressed hash.
///
/// Computes the hash via `fragmentation.hash_fragment`, serializes via
/// `fragmentation.serialize`, then writes to `<dir>/<hash>`.
/// Returns Ok(Nil) on success, Error(FileError) on failure.
/// Idempotent: writing the same fragment twice produces the same file.
pub fn write(
  fragment: fragmentation.Fragment(data),
  dir: String,
  encode: fn(data) -> String,
) -> Result(Nil, simplifile.FileError) {
  let hash = fragmentation.hash_fragment(fragment, encode)
  let content = fragmentation.serialize(fragment, encode)
  let path = dir <> "/" <> hash
  simplifile.write(path, content)
}

/// Convenience: write a Fragment(String) with identity encoder.
pub fn write_string(
  fragment: fragmentation.Fragment(String),
  dir: String,
) -> Result(Nil, simplifile.FileError) {
  write(fragment, dir, fn(s) { s })
}
