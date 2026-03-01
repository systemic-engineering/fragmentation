import fragmentation
import fragmentation/git
import gleeunit/should
import simplifile

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_witnessed() -> fragmentation.Witnessed {
  fragmentation.witnessed(
    fragmentation.author("alex"),
    fragmentation.committer("reed"),
    fragmentation.timestamp("2026-03-01T00:00:00Z"),
    fragmentation.message("test"),
  )
}

fn make_shard(data: String) -> fragmentation.Fragment {
  let r = fragmentation.ref(fragmentation.hash(data), "self")
  fragmentation.shard(r, test_witnessed(), data)
}

// ---------------------------------------------------------------------------
// write_fragment_creates_file_test
//
// write_fragment writes a file named by SHA containing serialized fragment.
// ---------------------------------------------------------------------------

pub fn write_fragment_creates_file_test() {
  let dir = "/tmp/fragmentation_git_test_write"
  let _ = simplifile.create_directory(dir)
  let frag = make_shard("hello-world")
  let sha = fragmentation.hash_fragment(frag)

  let result = git.write(frag, dir)
  result |> should.be_ok()

  let path = dir <> "/" <> sha
  simplifile.is_file(path) |> should.equal(Ok(True))
}

// ---------------------------------------------------------------------------
// write_fragment_idempotent_test
//
// write_fragment is idempotent — same SHA, same content, no error on repeat.
// ---------------------------------------------------------------------------

pub fn write_fragment_idempotent_test() {
  let dir = "/tmp/fragmentation_git_test_idempotent"
  let _ = simplifile.create_directory(dir)
  let frag = make_shard("idempotent-shard")
  let sha = fragmentation.hash_fragment(frag)

  let r1 = git.write(frag, dir)
  let r2 = git.write(frag, dir)

  r1 |> should.be_ok()
  r2 |> should.be_ok()

  // Only one file: idempotent means same path, same content
  let path = dir <> "/" <> sha
  simplifile.is_file(path) |> should.equal(Ok(True))
}

// ---------------------------------------------------------------------------
// write_two_fragments_test
//
// Writing two different fragments creates two files.
// ---------------------------------------------------------------------------

pub fn write_two_fragments_test() {
  let dir = "/tmp/fragmentation_git_test_two"
  let _ = simplifile.create_directory(dir)
  let frag_a = make_shard("fragment-alpha")
  let frag_b = make_shard("fragment-beta")
  let sha_a = fragmentation.hash_fragment(frag_a)
  let sha_b = fragmentation.hash_fragment(frag_b)

  git.write(frag_a, dir) |> should.be_ok()
  git.write(frag_b, dir) |> should.be_ok()

  // SHAs must be distinct
  sha_a |> should.not_equal(sha_b)

  simplifile.is_file(dir <> "/" <> sha_a) |> should.equal(Ok(True))
  simplifile.is_file(dir <> "/" <> sha_b) |> should.equal(Ok(True))
}
