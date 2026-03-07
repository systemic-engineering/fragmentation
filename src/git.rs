use std::fs;
use std::io;
use std::path::Path;

use crate::fragment::{self, Fragment};

/// Write a fragment to disk under `dir`, named by its content-addressed SHA.
///
/// Computes the SHA via `hash_fragment`, serializes via `serialize`,
/// then writes to `<dir>/<sha>`.
/// Idempotent: writing the same fragment twice produces the same file.
pub fn write(fragment: &Fragment, dir: &str) -> io::Result<()> {
    let sha = fragment::hash_fragment(fragment);
    let content = fragment::serialize(fragment);
    let path = Path::new(dir).join(&sha);
    fs::write(path, content)
}
