use std::process::Command;

use fragmentation::encoding;
use fragmentation::fragment;

#[test]
fn shard_prints_blob_oid() {
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["shard", "hello"])
        .output()
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let oid = String::from_utf8(output.stdout).unwrap().trim().to_string();
    assert_eq!(oid, fragment::blob_oid("hello"));
}

#[test]
fn shard_reads_stdin() {
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["shard"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"hello").unwrap();
            child.wait_with_output()
        })
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let oid = String::from_utf8(output.stdout).unwrap().trim().to_string();
    assert_eq!(oid, fragment::blob_oid("hello"));
}

#[test]
fn fractal_prints_tree_oid() {
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["fractal", "hello world"])
        .output()
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let oid = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let tree = encoding::encode("hello world");
    assert_eq!(oid, fragment::content_oid(&tree));
}

#[test]
fn fractal_reads_stdin() {
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["fractal"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"hello world")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let oid = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let tree = encoding::encode("hello world");
    assert_eq!(oid, fragment::content_oid(&tree));
}

fn init_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    Command::new("git")
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "config",
            "user.name",
            "test",
        ])
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "config",
            "user.email",
            "test@test.local",
        ])
        .output()
        .unwrap();
    dir
}

#[test]
fn commit_writes_root_to_repo() {
    let dir = init_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args([
            "commit",
            "--repo",
            dir.path().to_str().unwrap(),
            "--message",
            "first observation",
            "hello world",
        ])
        .output()
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sha = String::from_utf8(output.stdout).unwrap().trim().to_string();
    assert_eq!(sha.len(), 40, "expected SHA-1 hex, got: {}", sha);
}

#[test]
fn commit_child_has_parent() {
    let dir = init_repo();

    // root commit
    let root_output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args([
            "commit",
            "--repo",
            dir.path().to_str().unwrap(),
            "--message",
            "root",
            "hello",
        ])
        .output()
        .expect("failed to run fragmentation");
    assert!(root_output.status.success());
    let root_sha = String::from_utf8(root_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // child commit
    let child_output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args([
            "commit",
            "--repo",
            dir.path().to_str().unwrap(),
            "--message",
            "child",
            "--parent",
            &root_sha,
            "hello updated",
        ])
        .output()
        .expect("failed to run fragmentation");
    assert!(
        child_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&child_output.stderr)
    );
    let child_sha = String::from_utf8(child_output.stdout)
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(child_sha.len(), 40);
    assert_ne!(child_sha, root_sha);

    // verify parent via git cat-file
    let log = Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "cat-file",
            "-p",
            &child_sha,
        ])
        .output()
        .unwrap();
    let log_str = String::from_utf8(log.stdout).unwrap();
    assert!(
        log_str.contains(&format!("parent {}", root_sha)),
        "commit should reference parent"
    );
}

// ===========================================================================
// sign — visibility layer
// ===========================================================================

#[test]
fn sign_prints_empty_hex_without_signing_config() {
    // No gpg.format configured → Local::None → empty signature
    let dir = init_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["sign", "--repo", dir.path().to_str().unwrap(), "hello"])
        .output()
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sig_hex = String::from_utf8(output.stdout).unwrap().trim().to_string();
    // Local::None produces empty signature bytes → empty hex string
    assert!(
        sig_hex.is_empty(),
        "plain sign should produce empty signature, got: {}",
        sig_hex
    );
}

#[test]
fn sign_reads_stdin() {
    let dir = init_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["sign", "--repo", dir.path().to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"hello").unwrap();
            child.wait_with_output()
        })
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ===========================================================================
// encrypt / decrypt — visibility layer roundtrip
// ===========================================================================

#[test]
fn encrypt_decrypt_roundtrip_plain() {
    // Local::None → plaintext passthrough, but the subcommands work
    let dir = init_repo();
    let encrypt_output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args([
            "encrypt",
            "--repo",
            dir.path().to_str().unwrap(),
            "secret message",
        ])
        .output()
        .expect("failed to run fragmentation");

    assert!(
        encrypt_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&encrypt_output.stderr)
    );
    let ciphertext = encrypt_output.stdout; // raw bytes
    assert!(!ciphertext.is_empty(), "ciphertext should not be empty");

    // Decrypt: pipe ciphertext into stdin
    let decrypt_output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["decrypt", "--repo", dir.path().to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(&ciphertext).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run fragmentation");

    assert!(
        decrypt_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&decrypt_output.stderr)
    );
    let plaintext = String::from_utf8(decrypt_output.stdout).unwrap();
    assert_eq!(plaintext.trim(), "secret message");
}

#[test]
fn encrypt_reads_stdin() {
    let dir = init_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["encrypt", "--repo", dir.path().to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"hello").unwrap();
            child.wait_with_output()
        })
        .expect("failed to run fragmentation");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
}

// ===========================================================================
// Error paths — open_repo and detect_keys failure branches
// ===========================================================================

#[test]
fn decrypt_fails_with_invalid_utf8_ciphertext() {
    // Local::None decrypt: PlainKeys passes ciphertext directly to String::decode.
    // Invalid UTF-8 bytes → LocalError::Decode → process::exit(1) (main.rs:250-251).
    let dir = init_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["decrypt", "--repo", dir.path().to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            // \xff\xfe is not valid UTF-8
            child.stdin.take().unwrap().write_all(b"\xff\xfe").unwrap();
            child.wait_with_output()
        })
        .expect("failed to spawn fragmentation");
    assert!(
        !output.status.success(),
        "should fail with invalid UTF-8 ciphertext"
    );
}

#[test]
fn commit_fails_with_invalid_repo_path() {
    // open_repo calls process::exit(1) when git2::Repository::open fails
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args([
            "commit",
            "--repo",
            "/nonexistent/not/a/git/repo",
            "--message",
            "test",
            "data",
        ])
        .output()
        .expect("failed to spawn fragmentation");
    assert!(
        !output.status.success(),
        "should fail for non-git directory"
    );
}

#[test]
fn commit_fails_with_nonexistent_parent_sha() {
    // Draft::write fails when parent SHA doesn't exist → process::exit(1) (main.rs:182-183)
    let dir = init_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args([
            "commit",
            "--repo",
            dir.path().to_str().unwrap(),
            "--message",
            "test",
            "--parent",
            "0000000000000000000000000000000000000000",
            "data",
        ])
        .output()
        .expect("failed to spawn fragmentation");
    assert!(
        !output.status.success(),
        "should fail with nonexistent parent SHA"
    );
}

#[cfg(feature = "ssh")]
#[test]
fn sign_fails_with_missing_ssh_key_file() {
    // detect_keys calls process::exit(1) when SSH::from_path fails
    let dir = init_repo();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "config",
            "gpg.format",
            "ssh",
        ])
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "config",
            "user.signingkey",
            "/nonexistent/path/to/ssh/key",
        ])
        .output()
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["sign", "--repo", dir.path().to_str().unwrap(), "hello"])
        .output()
        .expect("failed to spawn fragmentation");
    assert!(
        !output.status.success(),
        "should fail with missing key file"
    );
}

#[cfg(feature = "ssh")]
#[test]
fn sign_with_ssh_key_produces_hex_output() {
    // local.sign() with SSH key produces non-empty bytes → print!("{}", hex::encode(bytes))
    use fragmentation::keys::SSH;
    let dir = init_repo();
    let key = SSH::generate_ed25519().expect("generate test key");
    let key_path = dir.path().join("test_signing_key");
    key.write_to_file(&key_path).unwrap();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "config",
            "gpg.format",
            "ssh",
        ])
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "config",
            "user.signingkey",
            key_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fragmentation"))
        .args(["sign", "--repo", dir.path().to_str().unwrap(), "hello"])
        .output()
        .expect("failed to spawn fragmentation");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sig_hex = String::from_utf8(output.stdout).unwrap();
    let sig_hex = sig_hex.trim();
    assert!(!sig_hex.is_empty(), "SSH sign should produce non-empty hex");
    assert!(
        sig_hex.chars().all(|c| c.is_ascii_hexdigit()),
        "output should be valid hex, got: {}",
        sig_hex
    );
}
