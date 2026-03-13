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
