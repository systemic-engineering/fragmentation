//! Append-only git-notes wrapper for spectral topic refs.
//!
//! Phase 5 of `docs/git-native-graph-plan.md` §3.6 / §6: high-frequency
//! out-of-band data — optimizer hot-paths, pressure events, scheduler
//! ticks, cherry-pick provenance — lives in git-notes under the
//! `refs/spectral/notes/<topic>` namespace.
//!
//! Unlike `git2::Repository::note(...)` which **overwrites** an existing
//! note for a given target, [`append_note`] concatenates new bodies onto
//! the existing note (separated by a newline). This gives us per-topic,
//! per-target append-only history.
//!
//! Reads via [`read_notes`] return one element per logical append in
//! chronological order (oldest first within a single note blob).

use git2::{Oid, Repository, Signature};

/// Errors from the notes wrapper.
#[derive(Debug)]
pub enum Error {
    /// libgit2 returned an error.
    Git(git2::Error),
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Error::Git(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Git(e) => write!(f, "git: {}", e),
        }
    }
}

impl std::error::Error for Error {}

/// Append `body` to the note at `ref_name` attached to `target_oid`.
///
/// `ref_name` must be a fully qualified notes ref (e.g.
/// `refs/spectral/notes/hot-paths`). The note's content becomes
/// `<existing>\n<body>` if a note already exists for `target_oid`,
/// otherwise just `<body>`. Returns the OID of the resulting note blob.
pub fn append_note(
    repo: &Repository,
    ref_name: &str,
    target_oid: Oid,
    body: &str,
) -> Result<Oid, Error> {
    let sig = Signature::now("spectral", "spectral@local")?;
    // Read existing note (if any) and concatenate.
    let merged = match repo.find_note(Some(ref_name), target_oid) {
        Ok(existing) => {
            let prev = existing.message().unwrap_or("");
            if prev.is_empty() {
                body.to_string()
            } else if prev.ends_with('\n') {
                format!("{}{}", prev, body)
            } else {
                format!("{}\n{}", prev, body)
            }
        }
        Err(_) => body.to_string(),
    };
    // `force=true` so we overwrite the existing note blob with the
    // concatenated content. The append semantics live in `merged`.
    let oid = repo.note(&sig, &sig, Some(ref_name), target_oid, &merged, true)?;
    Ok(oid)
}

/// Read all logical appends made via [`append_note`] for `target_oid`.
///
/// Returns the lines of the note split on `\n`, in chronological order.
/// A missing note (no entry exists) returns `Ok(vec![])`.
///
/// Rationale: each [`append_note`] call concatenates with a `\n`
/// separator, so splitting on `\n` recovers individual records. Records
/// that themselves contain newlines should be encoded by the caller (we
/// recommend a single-line wire format per topic — JSON without
/// whitespace, or `key=val` lines).
pub fn read_notes(
    repo: &Repository,
    ref_name: &str,
    target_oid: Oid,
) -> Result<Vec<String>, Error> {
    match repo.find_note(Some(ref_name), target_oid) {
        Ok(note) => {
            let msg = note.message().unwrap_or("");
            if msg.is_empty() {
                return Ok(Vec::new());
            }
            Ok(msg
                .split('\n')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect())
        }
        Err(_) => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    fn empty_commit(repo: &Repository, message: &str) -> Oid {
        let sig = Signature::now("test", "test@local").unwrap();
        let tb = repo.treebuilder(None).unwrap();
        let tree_oid = tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(None, &sig, &sig, message, &tree, &[]).unwrap()
    }

    #[test]
    fn missing_note_returns_empty() {
        let (_dir, repo) = fresh_repo();
        let target = empty_commit(&repo, "anchor");
        let lines = read_notes(&repo, "refs/spectral/notes/test", target).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn append_new_note_creates_record() {
        let (_dir, repo) = fresh_repo();
        let target = empty_commit(&repo, "anchor");
        append_note(&repo, "refs/spectral/notes/test", target, "first record").unwrap();
        let lines = read_notes(&repo, "refs/spectral/notes/test", target).unwrap();
        assert_eq!(lines, vec!["first record"]);
    }

    #[test]
    fn append_second_note_concatenates() {
        let (_dir, repo) = fresh_repo();
        let target = empty_commit(&repo, "anchor");
        append_note(&repo, "refs/spectral/notes/test", target, "first").unwrap();
        append_note(&repo, "refs/spectral/notes/test", target, "second").unwrap();
        append_note(&repo, "refs/spectral/notes/test", target, "third").unwrap();
        let lines = read_notes(&repo, "refs/spectral/notes/test", target).unwrap();
        assert_eq!(lines, vec!["first", "second", "third"]);
    }

    #[test]
    fn read_notes_returns_chronological_order() {
        let (_dir, repo) = fresh_repo();
        let target = empty_commit(&repo, "anchor");
        for i in 0..5 {
            append_note(
                &repo,
                "refs/spectral/notes/ticks",
                target,
                &format!("tick={i}"),
            )
            .unwrap();
        }
        let lines = read_notes(&repo, "refs/spectral/notes/ticks", target).unwrap();
        assert_eq!(
            lines,
            vec!["tick=0", "tick=1", "tick=2", "tick=3", "tick=4"]
        );
    }

    #[test]
    fn append_returns_oid() {
        let (_dir, repo) = fresh_repo();
        let target = empty_commit(&repo, "anchor");
        let oid1 = append_note(&repo, "refs/spectral/notes/test", target, "a").unwrap();
        assert!(!oid1.is_zero());
        // Second append produces a different note blob OID (concat → new content).
        let oid2 = append_note(&repo, "refs/spectral/notes/test", target, "b").unwrap();
        assert_ne!(oid1, oid2);
    }

    #[test]
    fn separate_targets_are_independent() {
        let (_dir, repo) = fresh_repo();
        let t1 = empty_commit(&repo, "first");
        let t2 = empty_commit(&repo, "second");
        append_note(&repo, "refs/spectral/notes/test", t1, "for-t1").unwrap();
        append_note(&repo, "refs/spectral/notes/test", t2, "for-t2").unwrap();
        let lines1 = read_notes(&repo, "refs/spectral/notes/test", t1).unwrap();
        let lines2 = read_notes(&repo, "refs/spectral/notes/test", t2).unwrap();
        assert_eq!(lines1, vec!["for-t1"]);
        assert_eq!(lines2, vec!["for-t2"]);
    }
}
