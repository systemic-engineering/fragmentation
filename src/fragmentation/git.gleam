/// git: content-addressed fragment persistence.
///
/// Writes fragments to disk named by their SHA.
/// The store is a directory. Each fragment becomes a file.
/// File name = SHA-256 of canonical serialization.
/// Idempotent: same SHA, same content, same file.
import fragmentation
import simplifile

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// Write a fragment to disk under `dir`, named by its content-addressed SHA.
///
/// Computes the SHA via `fragmentation.hash_fragment`, serializes via
/// `fragmentation.serialize`, then writes to `<dir>/<sha>`.
/// Returns Ok(Nil) on success, Error(FileError) on failure.
/// Idempotent: writing the same fragment twice produces the same file.
pub fn write(
  fragment: fragmentation.Fragment,
  dir: String,
) -> Result(Nil, simplifile.FileError) {
  let sha = fragmentation.hash_fragment(fragment)
  let content = fragmentation.serialize(fragment)
  let path = dir <> "/" <> sha
  simplifile.write(path, content)
}
