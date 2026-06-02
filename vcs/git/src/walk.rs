//! Commit walking: `git log --follow` over a path prefix.
//!
//! `walk_commits_following` is the backing for `memory_blame`. Starting at
//! `start_ref`, it walks the commit history and yields every commit whose
//! tree differs from its first parent's tree under the given path prefix.
//! Root commits are emitted iff their tree contains anything under the
//! prefix.
//!
//! Phase 4 of `docs/git-native-graph-plan.md` §6.

use git2::{DiffOptions, Oid, Repository, Sort};

/// Walk commits reachable from `start_ref` and return those whose tree
/// changes under `path_prefix`.
///
/// `path_prefix` is a directory-style prefix like `nodes/<oid>/`. A trailing
/// slash is recommended; `git2`'s pathspec treats it as a directory match.
///
/// Order: newest-first (Topological + Time-ordered, matching `git log`).
/// Each tuple is `(commit_oid, summary)`. A commit is included when:
/// - it has at least one parent and the diff against the first parent
///   contains a delta whose new-file path starts with `path_prefix`, or
/// - it has zero parents (root) and its tree has at least one entry under
///   `path_prefix`.
pub fn walk_commits_following(
    repo: &Repository,
    start_ref: &str,
    path_prefix: &str,
) -> Result<Vec<(Oid, String)>, git2::Error> {
    let reference = repo.find_reference(start_ref)?;
    let resolved = reference.resolve().unwrap_or(reference);
    let start_oid = match resolved.target() {
        Some(o) => o,
        None => return Ok(Vec::new()),
    };

    let mut walker = repo.revwalk()?;
    walker.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;
    walker.push(start_oid)?;

    let mut out = Vec::new();
    for oid_res in walker {
        let oid = oid_res?;
        let commit = repo.find_commit(oid)?;
        let summary = commit.summary().unwrap_or("").to_string();
        let tree = commit.tree()?;

        let touched = if commit.parent_count() == 0 {
            tree_has_prefix(repo, &tree, path_prefix)?
        } else {
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let mut opts = DiffOptions::new();
            opts.pathspec(path_prefix);
            let diff =
                repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opts))?;
            let mut hit = false;
            diff.foreach(
                &mut |delta, _| {
                    let matches = delta
                        .new_file()
                        .path()
                        .map(|p| p.to_string_lossy().starts_with(path_prefix))
                        .unwrap_or(false)
                        || delta
                            .old_file()
                            .path()
                            .map(|p| p.to_string_lossy().starts_with(path_prefix))
                            .unwrap_or(false);
                    if matches {
                        hit = true;
                    }
                    true
                },
                None,
                None,
                None,
            )?;
            hit
        };

        if touched {
            out.push((oid, summary));
        }
    }
    Ok(out)
}

/// Return true iff the tree contains any entry under `path_prefix`.
fn tree_has_prefix(
    repo: &Repository,
    tree: &git2::Tree<'_>,
    path_prefix: &str,
) -> Result<bool, git2::Error> {
    let mut found = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        let name = entry.name().unwrap_or("");
        let full = format!("{dir}{name}");
        if full.starts_with(path_prefix) || path_prefix.starts_with(&full) {
            // exact-or-descendant match
            if full.starts_with(path_prefix) && full != path_prefix.trim_end_matches('/') {
                found = true;
                return git2::TreeWalkResult::Abort;
            }
        }
        git2::TreeWalkResult::Ok
    })?;
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, Repository) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        (tmp, repo)
    }

    /// Create a commit whose tree contains `<prefix>/marker` blob with the
    /// given content, and (if `extra_a`) a sibling node A blob.
    /// Parents are linked.
    fn write_node_commit(
        repo: &Repository,
        node_path: &str,
        marker_content: &[u8],
        parent: Option<Oid>,
        msg: &str,
    ) -> Oid {
        // Build a tree with `nodes/<node>/.content`
        let mut nodes_tb = repo.treebuilder(None).unwrap();
        // If parent exists, we want to preserve other nodes/* subtrees.
        if let Some(p_oid) = parent {
            let parent_commit = repo.find_commit(p_oid).unwrap();
            let parent_tree = parent_commit.tree().unwrap();
            let nodes_id = parent_tree.get_name("nodes").map(|e| e.id());
            if let Some(nid) = nodes_id {
                let nodes_tree = repo.find_tree(nid).unwrap();
                for e in nodes_tree.iter() {
                    nodes_tb
                        .insert(e.name().unwrap(), e.id(), e.filemode())
                        .unwrap();
                }
            }
        }
        let marker = repo.blob(marker_content).unwrap();
        let mut node_tb = repo.treebuilder(None).unwrap();
        node_tb.insert(".content", marker, 0o100644).unwrap();
        let node_tree = node_tb.write().unwrap();
        nodes_tb.insert(node_path, node_tree, 0o040000).unwrap();
        let nodes_tree = nodes_tb.write().unwrap();

        let mut root_tb = repo.treebuilder(None).unwrap();
        root_tb.insert("nodes", nodes_tree, 0o040000).unwrap();
        let tree_oid = root_tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@local").unwrap();
        let parent_commits: Vec<git2::Commit> = parent
            .into_iter()
            .map(|p| repo.find_commit(p).unwrap())
            .collect();
        let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();
        repo.commit(None, &sig, &sig, msg, &tree, &parent_refs)
            .unwrap()
    }

    #[test]
    fn walk_commits_following_filters_by_path() {
        let (_tmp, repo) = setup();
        // First commit edits node A.
        let c1 = write_node_commit(&repo, "oid_a", b"first-A", None, "edit A");
        // Second commit edits node B.
        let c2 = write_node_commit(&repo, "oid_b", b"first-B", Some(c1), "edit B");

        repo.reference("refs/spectral/HEAD", c2, true, "test").unwrap();

        let only_a =
            walk_commits_following(&repo, "refs/spectral/HEAD", "nodes/oid_a/").unwrap();
        assert_eq!(only_a.len(), 1, "exactly one commit touched node A");
        assert_eq!(only_a[0].0, c1);

        let only_b =
            walk_commits_following(&repo, "refs/spectral/HEAD", "nodes/oid_b/").unwrap();
        assert_eq!(only_b.len(), 1, "exactly one commit touched node B");
        assert_eq!(only_b[0].0, c2);
    }

    #[test]
    fn walk_commits_following_missing_ref_returns_empty() {
        let (_tmp, repo) = setup();
        let res = walk_commits_following(&repo, "refs/does/not/exist", "nodes/whatever/");
        assert!(res.is_err());
    }
}
