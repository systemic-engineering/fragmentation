# fragmentation-git

Git transport layer for fragmentation content store. Workspace member of `fragmentation`.

## Provenance

Folded in **2026-05-24** from the standalone `../fragmentation-git/` repo
(originally extracted in commit `f1e1135`). Per
`docs/specs/mirror-native-vcs.md` §4.5: VCS adapters live inside
fragmentation under `vcs/`, not as separate sibling repos.

The standalone `../fragmentation-git/` repo is **retired** after this merge
(Alex archives it manually after verification). Two files in this directory
were brought in from the standalone repo as a clean copy:

- `src/atomic.rs`, `src/bounded_store.rs`, `src/commit.rs`,
  `src/concurrent_store.rs`, `src/git.rs`, `src/namespaced.rs`, `src/notes.rs`,
  `src/store.rs`, `src/walk.rs`

C3 of T1 will additionally move `git.rs`, `fuse.rs`, and the CLI
(`main.rs` → `src/bin/frgmt-git.rs`) out of `fragmentation/src/` and into
this crate. After C3, this crate owns *all* git-flavored code; the
substrate (`fragmentation`) carries no `git2` import.

## Layout (after T1)

```
vcs/git/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── atomic.rs           (atomic ref writes)
    ├── bounded_store.rs    (GitBoundedStore)
    ├── commit.rs           (DraftWriteExt + git commit format)
    ├── concurrent_store.rs (ConcurrentStoreGitExt)
    ├── git.rs              (write_tree, read_node, write_commit — low-level git ops)
    ├── fuse.rs             (FUSE portal — added by C3)
    ├── namespaced.rs       (namespaced refs)
    ├── notes.rs            (git-notes for fragmentation metadata)
    ├── store.rs            (GitStore)
    ├── walk.rs             (git-walk extension)
    └── bin/
        └── frgmt-git.rs    (CLI — moved from src/main.rs by C3)
```
