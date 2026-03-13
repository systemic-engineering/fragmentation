use clap::{Parser, Subcommand};
use std::io::Read;

use fragmentation::fragment::Fragmentable;
use fragmentation::{encoding, fragment, keys};

#[derive(Parser)]
#[command(name = "fragmentation")]
#[command(about = "Content-addressed fragment trees, git-native")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compute shard (blob) OID.
    Shard {
        /// Data to shard. Reads stdin if omitted.
        data: Option<String>,
    },
    /// Encode text into a fragment tree. Prints the root OID.
    Fractal {
        /// Text to encode. Reads stdin if omitted.
        data: Option<String>,
    },
    /// Encode text, write fragment tree + commit to a git repo. Prints commit SHA.
    #[cfg(feature = "git")]
    Commit {
        /// Text to commit. Reads stdin if omitted.
        data: Option<String>,
        /// Commit message.
        #[arg(short, long)]
        message: String,
        /// Path to git repository. Defaults to current directory.
        #[arg(short, long)]
        repo: Option<String>,
        /// Parent commit SHA. Omit for root commit.
        #[arg(short, long)]
        parent: Option<String>,
        /// Ref name under refs/fragmentation/. Defaults to "default".
        #[arg(long = "ref", default_value = "default")]
        ref_name: String,
    },
    /// Sign a shard. Prints signature bytes as hex.
    #[cfg(feature = "git")]
    Sign {
        /// Data to sign. Reads stdin if omitted.
        data: Option<String>,
        /// Path to git repository (for key detection). Defaults to current directory.
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Encrypt a shard. Writes ciphertext to stdout.
    #[cfg(feature = "git")]
    Encrypt {
        /// Data to encrypt. Reads stdin if omitted.
        data: Option<String>,
        /// Path to git repository (for key detection). Defaults to current directory.
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Decrypt ciphertext from stdin. Writes plaintext to stdout.
    #[cfg(feature = "git")]
    Decrypt {
        /// Path to git repository (for key detection). Defaults to current directory.
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Mount a FUSE filesystem backed by fragmentation.
    /// Blocks until unmounted (fusermount -u <mountpoint> or umount on macOS).
    #[cfg(feature = "fuse-mount")]
    Mount {
        /// Directory to mount.
        mountpoint: String,
        /// Path to git repository. Defaults to current directory.
        #[arg(short, long)]
        repo: Option<String>,
        /// Ref name under refs/fragmentation/. Defaults to "default".
        #[arg(long = "ref", default_value = "default")]
        ref_name: String,
    },
    /// Run as a git smudge/clean filter (identity transform).
    #[cfg(feature = "fuse-mount")]
    Filter {
        /// Smudge: git → working tree (read from stdin, write to stdout).
        #[arg(long)]
        smudge: bool,
        /// Clean: working tree → git (read from stdin, write to stdout).
        #[arg(long)]
        clean: bool,
        /// Path to git repository. Defaults to current directory.
        #[arg(short, long)]
        repo: Option<String>,
    },
}

#[cfg(feature = "git")]
fn open_repo(repo: Option<String>) -> git2::Repository {
    let repo_path = repo.unwrap_or_else(|| ".".to_string());
    git2::Repository::open(&repo_path).unwrap_or_else(|e| {
        eprintln!("failed to open repo at {}: {}", repo_path, e);
        std::process::exit(1);
    })
}

#[cfg(feature = "git")]
fn detect_keys(repo: &git2::Repository) -> keys::Local {
    keys::Local::from_repo(repo).unwrap_or_else(|e| {
        eprintln!("failed to detect keys: {}", e);
        std::process::exit(1);
    })
}

fn read_input(data: Option<String>) -> String {
    match data {
        Some(d) => d,
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .expect("failed to read stdin");
            buf
        }
    }
}

fn read_stdin_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buf)
        .expect("failed to read stdin");
    buf
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Shard { data } => {
            let input = read_input(data);
            println!("{}", fragment::blob_oid(&input));
        }
        Command::Fractal { data } => {
            let input = read_input(data);
            let tree = encoding::encode(&input);
            println!("{}", fragment::content_oid(&tree));
        }
        #[cfg(feature = "git")]
        Command::Commit {
            data,
            message,
            repo,
            parent,
            ref_name,
        } => {
            let input = read_input(data);
            let tree = encoding::encode(&input);
            let repository = open_repo(repo);

            let config = repository.config().expect("failed to read git config");
            let name = config
                .get_string("user.name")
                .expect("git config user.name not set");
            let email = config
                .get_string("user.email")
                .expect("git config user.email not set");

            let author = fragmentation::witnessed::Author::new(&name, &email);
            let committer = fragmentation::witnessed::Committer::new(&name, &email);

            let parent_sha = parent.map(fragmentation::sha::Sha);
            let draft = match parent_sha {
                None => fragmentation::commit::Draft::root(&message, tree),
                Some(ref p) => fragmentation::commit::Draft::new(
                    &message,
                    tree,
                    fragmentation::commit::Parent(p.clone()),
                ),
            };

            let commit = draft
                .authored(author)
                .write(&repository, committer)
                .unwrap_or_else(|e| {
                    eprintln!("failed to write commit: {}", e);
                    std::process::exit(1);
                });

            let full_ref = format!("refs/fragmentation/{}", ref_name);
            let oid = git2::Oid::from_str(&commit.sha().0).expect("invalid oid");
            repository
                .reference(&full_ref, oid, true, "fragmentation commit")
                .unwrap_or_else(|e| {
                    eprintln!("failed to update ref: {}", e);
                    std::process::exit(1);
                });

            println!("{}", commit.sha().0);
        }
        #[cfg(feature = "git")]
        Command::Sign { data, repo } => {
            use fragmentation::fragment::{Blob, Fractal};
            use keys::Keys;

            let input = read_input(data);
            let repository = open_repo(repo);
            let local = detect_keys(&repository);

            let shard: Fractal<Blob> = Fractal::shard_typed(
                fragmentation::ref_::Ref::new(
                    fragmentation::sha::Sha(fragment::blob_oid(&input)),
                    "self",
                ),
                input.into_bytes(),
            );
            let sig = local.sign(&shard).unwrap_or_else(|e| {
                eprintln!("signing failed: {}", e);
                std::process::exit(1);
            });
            let bytes = sig.bytes();
            if !bytes.is_empty() {
                print!("{}", hex::encode(bytes));
            }
        }
        #[cfg(feature = "git")]
        Command::Encrypt { data, repo } => {
            use fragmentation::fragment::Fractal;
            use keys::Keys;
            use std::io::Write;

            let input = read_input(data);
            let repository = open_repo(repo);
            let local = detect_keys(&repository);

            let shard = Fractal::shard(
                fragmentation::ref_::Ref::new(
                    fragmentation::sha::Sha(fragment::blob_oid(&input)),
                    "self",
                ),
                &input,
            );
            let encrypted = local.encrypt(shard).unwrap_or_else(|e| {
                eprintln!("encryption failed: {}", e);
                std::process::exit(1);
            });
            std::io::stdout()
                .write_all(encrypted.ciphertext())
                .expect("failed to write ciphertext");
        }
        #[cfg(feature = "git")]
        Command::Decrypt { repo } => {
            use fragmentation::fragment::Fractal;
            use keys::{Encrypted, Keys};
            use std::io::Write;

            let ciphertext = read_stdin_bytes();
            let repository = open_repo(repo);
            let local = detect_keys(&repository);

            let encrypted = Encrypted::new(ciphertext, local.clone());
            let decrypted: Fractal<String> = local.decrypt(&encrypted).unwrap_or_else(|e| {
                eprintln!("decryption failed: {}", e);
                std::process::exit(1);
            });
            std::io::stdout()
                .write_all(decrypted.data().as_bytes())
                .expect("failed to write plaintext");
        }
        #[cfg(feature = "fuse-mount")]
        Command::Mount {
            mountpoint,
            repo,
            ref_name,
        } => {
            let repository = open_repo(repo);
            let config = repository.config().expect("failed to read git config");
            let name = config
                .get_string("user.name")
                .expect("git config user.name not set");
            let email = config
                .get_string("user.email")
                .expect("git config user.email not set");
            let committer = fragmentation::witnessed::Committer::new(&name, &email);
            let full_ref = format!("refs/fragmentation/{}", ref_name);
            let fs = fragmentation::fuse::FragmentFs::open(repository, committer, full_ref);
            fuser::mount2(
                fs,
                mountpoint,
                &[
                    fuser::MountOption::RW,
                    fuser::MountOption::FSName("fragmentation".to_string()),
                ],
            )
            .unwrap_or_else(|e| {
                eprintln!("mount failed: {}", e);
                std::process::exit(1);
            });
        }
        #[cfg(feature = "fuse-mount")]
        Command::Filter {
            smudge: _,
            clean: _,
            repo: _,
        } => {
            // Identity transform: pass stdin → stdout unchanged.
            // First pass: witness protocol without transformation.
            use std::io::Write;
            let data = read_stdin_bytes();
            std::io::stdout()
                .write_all(&data)
                .expect("failed to write filter output");
        }
    }
}
