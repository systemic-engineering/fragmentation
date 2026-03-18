//! FUSE state machine tests — operate on FsInner directly, no kernel mount required.

#[cfg(feature = "fuse")]
mod fuse_state_tests {
    use fragmentation::fuse::{FsError, FsInner};
    use fragmentation::witnessed::Committer;

    fn make_inner(ref_name: &str) -> (tempfile::TempDir, FsInner) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let committer = Committer::new("test", "test@test");
        let inner = FsInner::new(repo, committer, ref_name.to_string());
        (dir, inner)
    }

    #[test]
    fn new_fs_has_root_inode() {
        let (_dir, inner) = make_inner("refs/fragmentation/test");
        assert!(inner.has_inode(1), "root inode should be 1");
        assert!(inner.is_dir(1), "root should be a directory");
    }

    #[test]
    fn create_file_allocates_inode() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, _fh) = inner.create_file(1, "test.txt").unwrap();
        assert!(ino > 1);
        assert!(inner.has_inode(ino));
    }

    #[test]
    fn write_accumulates_buffer() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (_ino, fh) = inner.create_file(1, "test.txt").unwrap();
        inner.write_to(fh, 0, b"hello world").unwrap();
        let buf = inner.read_buffer(fh).to_vec();
        assert_eq!(buf, b"hello world");
    }

    #[test]
    fn flush_creates_git_commit() {
        let (dir, mut inner) = make_inner("refs/fragmentation/test");
        let (_ino, fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(fh, 0, b"content").unwrap();
        inner.flush(fh, "fuse: file.txt").unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        let ref_result = repo.find_reference("refs/fragmentation/test");
        assert!(
            ref_result.is_ok(),
            "fragmentation ref should exist after flush"
        );
    }

    #[test]
    fn flush_advances_head() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        assert!(inner.head().is_none());
        let (_ino, fh) = inner.create_file(1, "a.txt").unwrap();
        inner.write_to(fh, 0, b"data").unwrap();
        inner.flush(fh, "fuse: a.txt").unwrap();
        assert!(inner.head().is_some());
    }

    #[test]
    fn flush_noop_when_not_dirty() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (_ino, fh) = inner.create_file(1, "a.txt").unwrap();
        // No write — not dirty
        inner.flush(fh, "fuse: a.txt").unwrap();
        assert!(inner.head().is_none(), "clean flush should not commit");
    }

    #[test]
    fn read_returns_committed_content() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "read.txt").unwrap();
        inner.write_to(fh, 0, b"readable content").unwrap();
        inner.flush(fh, "fuse: read.txt").unwrap();
        inner.release(fh);

        let content = inner.read_file(ino, 0, 1024).unwrap();
        assert_eq!(content, b"readable content");
    }

    #[test]
    fn mkdir_creates_directory() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let ino = inner.mkdir(1, "subdir").unwrap();
        assert!(ino > 1);
        assert!(inner.is_dir(ino));
    }

    #[test]
    fn unlink_removes_file() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, _fh) = inner.create_file(1, "to-remove.txt").unwrap();
        assert!(inner.has_inode(ino));
        inner.unlink(1, "to-remove.txt").unwrap();
        assert!(!inner.has_inode(ino));
    }

    #[test]
    fn flush_builds_fractal_from_inodes() {
        let (dir, mut inner) = make_inner("refs/fragmentation/test");
        let (_ino, fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(fh, 0, b"fractal content").unwrap();
        inner.flush(fh, "fuse: file.txt").unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        let ref_obj = repo.find_reference("refs/fragmentation/test").unwrap();
        let commit_oid = ref_obj.target().unwrap();
        let commit = repo.find_commit(commit_oid).unwrap();
        let tree_oid = commit.tree_id();

        let fractal = fragmentation::git::read_tree_named(&repo, tree_oid).unwrap();
        let file_child = fractal
            .children()
            .iter()
            .find(|c| c.self_ref().label == "file.txt");
        assert!(file_child.is_some(), "file.txt should be in fractal");
        use fragmentation::fragment::Fragmentable;
        assert_eq!(file_child.unwrap().data(), b"fractal content");
    }

    #[test]
    fn identical_content_same_blob_oid() {
        let (dir, mut inner) = make_inner("refs/fragmentation/test");

        let (_ino1, fh1) = inner.create_file(1, "a.txt").unwrap();
        inner.write_to(fh1, 0, b"same content").unwrap();
        inner.flush(fh1, "fuse: a.txt").unwrap();
        inner.release(fh1);

        let (_ino2, fh2) = inner.create_file(1, "b.txt").unwrap();
        inner.write_to(fh2, 0, b"same content").unwrap();
        inner.flush(fh2, "fuse: b.txt").unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        let ref_obj = repo.find_reference("refs/fragmentation/test").unwrap();
        let commit_oid = ref_obj.target().unwrap();
        let commit = repo.find_commit(commit_oid).unwrap();
        let tree = commit.tree().unwrap();

        let a_entry = tree.get_name("a.txt").unwrap();
        let b_entry = tree.get_name("b.txt").unwrap();
        assert_eq!(
            a_entry.id(),
            b_entry.id(),
            "identical content should dedup to same blob OID"
        );
    }

    #[test]
    fn commit_chain_has_correct_parents() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");

        let (_ino1, fh1) = inner.create_file(1, "first.txt").unwrap();
        inner.write_to(fh1, 0, b"first").unwrap();
        inner.flush(fh1, "fuse: first.txt").unwrap();
        let head1 = inner.head().unwrap();

        let (_ino2, fh2) = inner.create_file(1, "second.txt").unwrap();
        inner.write_to(fh2, 0, b"second").unwrap();
        inner.flush(fh2, "fuse: second.txt").unwrap();
        let head2 = inner.head().unwrap();

        assert_ne!(head1, head2);
        assert_eq!(
            inner.parent_of_head().unwrap(),
            head1,
            "second commit should have first as parent"
        );
    }

    #[test]
    fn mount_from_existing_ref_populates_inodes() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_owned();

        // Create initial state
        {
            let repo = git2::Repository::init(&repo_path).unwrap();
            let committer = Committer::new("test", "test@test");
            let mut inner = FsInner::new(repo, committer, "refs/fragmentation/test".to_string());
            let (_ino, fh) = inner.create_file(1, "existing.txt").unwrap();
            inner.write_to(fh, 0, b"existing content").unwrap();
            inner.flush(fh, "fuse: existing.txt").unwrap();
        }

        // Load from existing ref
        let repo = git2::Repository::open(&repo_path).unwrap();
        let committer = Committer::new("test", "test@test");
        let inner =
            FsInner::from_ref(repo, committer, "refs/fragmentation/test".to_string()).unwrap();

        assert!(
            inner.lookup_child(1, "existing.txt").is_some(),
            "existing.txt should be in root after from_ref"
        );
    }

    // -----------------------------------------------------------------------
    // FsError Display + From<git2::Error>
    // -----------------------------------------------------------------------

    #[test]
    fn fs_error_display_bad_fd() {
        assert_eq!(format!("{}", FsError::BadFd), "bad file descriptor");
    }

    #[test]
    fn fs_error_display_not_found() {
        assert_eq!(format!("{}", FsError::NotFound), "not found");
    }

    #[test]
    fn fs_error_display_not_a_file() {
        assert_eq!(format!("{}", FsError::NotAFile), "not a file");
    }

    #[test]
    fn fs_error_display_not_a_dir() {
        assert_eq!(format!("{}", FsError::NotADir), "not a directory");
    }

    #[test]
    fn fs_error_display_not_empty() {
        assert_eq!(format!("{}", FsError::NotEmpty), "directory not empty");
    }

    #[test]
    fn fs_error_display_other() {
        let e = FsError::Other("custom message".to_string());
        assert_eq!(format!("{}", e), "custom message");
    }

    #[test]
    fn fs_error_display_git_variant() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let git_err = repo.find_reference("refs/nonexistent").err().unwrap();
        let fs_err = FsError::Git(git_err);
        let msg = format!("{}", fs_err);
        assert!(msg.starts_with("git error:"));
    }

    #[test]
    fn fs_error_from_git2_error() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let git_err = repo.find_reference("refs/nonexistent").err().unwrap();
        let fs_err: FsError = git_err.into();
        assert!(matches!(fs_err, FsError::Git(_)));
    }

    // -----------------------------------------------------------------------
    // populate_from_fractal — directory branch (lines 163-164)
    // -----------------------------------------------------------------------

    #[test]
    fn from_ref_with_nested_directory_populates_tree() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_owned();

        {
            let repo = git2::Repository::init(&repo_path).unwrap();
            let committer = Committer::new("test", "test@test");
            let mut inner = FsInner::new(repo, committer, "refs/fragmentation/test".to_string());
            let subdir_ino = inner.mkdir(1, "subdir").unwrap();
            let (_ino, fh) = inner.create_file(subdir_ino, "nested.txt").unwrap();
            inner.write_to(fh, 0, b"nested content").unwrap();
            inner.flush(fh, "add nested dir").unwrap();
        }

        let repo = git2::Repository::open(&repo_path).unwrap();
        let committer = Committer::new("test", "test@test");
        let inner =
            FsInner::from_ref(repo, committer, "refs/fragmentation/test".to_string()).unwrap();

        let subdir_ino = inner
            .lookup_child(1, "subdir")
            .expect("subdir should exist");
        assert!(inner.is_dir(subdir_ino), "subdir should be a directory");
        assert!(
            inner.lookup_child(subdir_ino, "nested.txt").is_some(),
            "nested.txt should be inside subdir"
        );
    }

    // -----------------------------------------------------------------------
    // lookup_child on non-dir returns None (line 201)
    // -----------------------------------------------------------------------

    #[test]
    fn lookup_child_on_file_inode_returns_none() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (file_ino, _fh) = inner.create_file(1, "file.txt").unwrap();
        assert!(inner.lookup_child(file_ino, "child").is_none());
    }

    // -----------------------------------------------------------------------
    // read_buffer error paths (lines 208, 212)
    // -----------------------------------------------------------------------

    #[test]
    fn read_buffer_invalid_fh_returns_empty() {
        let (_dir, inner) = make_inner("refs/fragmentation/test");
        assert_eq!(inner.read_buffer(999), &[]);
    }

    #[test]
    fn read_buffer_dir_inode_returns_empty() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        // open_existing on the root dir — creates a fh pointing at a Dir
        let fh = inner.open_existing(1);
        assert_eq!(inner.read_buffer(fh), &[]);
    }

    // -----------------------------------------------------------------------
    // create_file on non-dir returns error (line 223)
    // -----------------------------------------------------------------------

    #[test]
    fn create_file_on_file_inode_returns_error() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (file_ino, _fh) = inner.create_file(1, "file.txt").unwrap();
        assert!(inner.create_file(file_ino, "child.txt").is_err());
    }

    // -----------------------------------------------------------------------
    // open_existing (lines 237-239)
    // -----------------------------------------------------------------------

    #[test]
    fn open_existing_creates_readable_handle() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, create_fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(create_fh, 0, b"data").unwrap();
        inner.release(create_fh);

        let open_fh = inner.open_existing(ino);
        assert_eq!(inner.read_buffer(open_fh), b"data");
    }

    // -----------------------------------------------------------------------
    // write_to error paths (lines 250, 253)
    // -----------------------------------------------------------------------

    #[test]
    fn write_to_dir_inode_returns_error() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        // open_existing on root dir — fh.ino points to a Dir
        let fh = inner.open_existing(1);
        assert!(inner.write_to(fh, 0, b"data").is_err());
    }

    #[test]
    fn write_to_sparse_pads_with_zeros() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (_ino, fh) = inner.create_file(1, "sparse.txt").unwrap();
        // Write at offset 5 with empty file — triggers the resize (line 253)
        inner.write_to(fh, 5, b"data").unwrap();
        let buf = inner.read_buffer(fh).to_vec();
        assert_eq!(buf.len(), 9);
        assert_eq!(&buf[..5], &[0u8; 5]);
        assert_eq!(&buf[5..], b"data");
    }

    // -----------------------------------------------------------------------
    // read_file error paths (lines 268, 273)
    // -----------------------------------------------------------------------

    #[test]
    fn read_file_offset_beyond_end_returns_empty() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(fh, 0, b"hello").unwrap();
        let result = inner.read_file(ino, 100, 1024).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn read_file_on_dir_returns_error() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        assert!(inner.read_file(1, 0, 1024).is_err());
    }

    // -----------------------------------------------------------------------
    // mkdir on non-dir (line 307)
    // -----------------------------------------------------------------------

    #[test]
    fn mkdir_on_file_inode_returns_error() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (file_ino, _fh) = inner.create_file(1, "file.txt").unwrap();
        assert!(inner.mkdir(file_ino, "subdir").is_err());
    }

    // -----------------------------------------------------------------------
    // unlink on non-dir (line 326)
    // -----------------------------------------------------------------------

    #[test]
    fn unlink_with_file_as_parent_returns_error() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (file_ino, _fh) = inner.create_file(1, "file.txt").unwrap();
        assert!(inner.unlink(file_ino, "anything").is_err());
    }

    // -----------------------------------------------------------------------
    // rmdir (lines 338-358)
    // -----------------------------------------------------------------------

    #[test]
    fn rmdir_removes_empty_directory() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let ino = inner.mkdir(1, "emptydir").unwrap();
        assert!(inner.has_inode(ino));
        inner.rmdir(1, "emptydir").unwrap();
        assert!(!inner.has_inode(ino));
        assert!(inner.lookup_child(1, "emptydir").is_none());
    }

    #[test]
    fn rmdir_with_file_as_parent_returns_error() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (file_ino, _fh) = inner.create_file(1, "file.txt").unwrap();
        assert!(inner.rmdir(file_ino, "anything").is_err());
    }

    #[test]
    fn rmdir_non_empty_directory_returns_error() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let dir_ino = inner.mkdir(1, "fulldir").unwrap();
        inner.create_file(dir_ino, "file.txt").unwrap();
        assert!(inner.rmdir(1, "fulldir").is_err());
    }

    // -----------------------------------------------------------------------
    // truncate (lines 361-369)
    // -----------------------------------------------------------------------

    #[test]
    fn truncate_shrinks_file_content() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(fh, 0, b"hello world").unwrap();
        inner.truncate(ino, 5);
        let content = inner.read_file(ino, 0, 1024).unwrap();
        assert_eq!(content, b"hello");
    }

    #[test]
    fn truncate_extends_file_with_zeros() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(fh, 0, b"hi").unwrap();
        inner.truncate(ino, 5);
        let content = inner.read_file(ino, 0, 1024).unwrap();
        assert_eq!(content, b"hi\0\0\0");
    }

    // -----------------------------------------------------------------------
    // fuse.rs line 351 — rmdir if-let false branch (ino is File, not Dir)
    // -----------------------------------------------------------------------

    #[test]
    fn rmdir_on_file_ino_succeeds() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        inner.create_file(1, "file.txt").unwrap();
        // rmdir on a File ino: if-let at fuse.rs:347 doesn't match Node::Dir →
        // skips the empty-check body → line 351 is the false-branch → proceeds to remove.
        inner.rmdir(1, "file.txt").unwrap();
        assert!(inner.lookup_child(1, "file.txt").is_none());
    }

    // -----------------------------------------------------------------------
    // fuse.rs line 368 — truncate if-let false branch (ino is Dir, not File)
    // -----------------------------------------------------------------------

    #[test]
    fn truncate_on_dir_ino_is_noop() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let dir_ino = inner.mkdir(1, "subdir").unwrap();
        // truncate on a Dir ino: if-let at fuse.rs:362 doesn't match Node::File →
        // skips body → line 368 is the false-branch → no-op.
        inner.truncate(dir_ino, 0);
        assert!(inner.is_dir(dir_ino));
    }

    // -----------------------------------------------------------------------
    // @read annotation — path_visibility
    // -----------------------------------------------------------------------

    #[test]
    fn path_visibility_private() {
        assert_eq!(
            fragmentation::fuse::path_visibility("private/keys/id.pub"),
            "private"
        );
    }

    #[test]
    fn path_visibility_private_with_leading_slash() {
        assert_eq!(
            fragmentation::fuse::path_visibility("/private/keys/id.pub"),
            "private"
        );
    }

    #[test]
    fn path_visibility_protected() {
        assert_eq!(
            fragmentation::fuse::path_visibility("protected/blog/draft.md"),
            "protected"
        );
    }

    #[test]
    fn path_visibility_protected_with_leading_slash() {
        assert_eq!(
            fragmentation::fuse::path_visibility("/protected/blog/draft.md"),
            "protected"
        );
    }

    #[test]
    fn path_visibility_public_default() {
        assert_eq!(
            fragmentation::fuse::path_visibility("songs/ballad.txt"),
            "public"
        );
    }

    #[test]
    fn path_visibility_public_root_file() {
        assert_eq!(fragmentation::fuse::path_visibility("README.md"), "public");
    }

    // -----------------------------------------------------------------------
    // @read annotation — ino_path
    // -----------------------------------------------------------------------

    #[test]
    fn ino_path_root_returns_none() {
        let (_dir, inner) = make_inner("refs/fragmentation/test");
        assert!(inner.ino_path(1).is_none(), "root inode has no path");
    }

    #[test]
    fn ino_path_simple_file() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, _fh) = inner.create_file(1, "hello.txt").unwrap();
        assert_eq!(inner.ino_path(ino).unwrap(), "hello.txt");
    }

    #[test]
    fn ino_path_nested_file() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let dir_ino = inner.mkdir(1, "private").unwrap();
        let (file_ino, _fh) = inner.create_file(dir_ino, "secret.txt").unwrap();
        assert_eq!(inner.ino_path(file_ino).unwrap(), "private/secret.txt");
    }

    #[test]
    fn ino_path_deeply_nested() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let a = inner.mkdir(1, "a").unwrap();
        let b = inner.mkdir(a, "b").unwrap();
        let (file_ino, _fh) = inner.create_file(b, "deep.txt").unwrap();
        assert_eq!(inner.ino_path(file_ino).unwrap(), "a/b/deep.txt");
    }

    // -----------------------------------------------------------------------
    // @read annotation — read_file creates annotations
    // -----------------------------------------------------------------------

    #[test]
    fn read_file_creates_annotation() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "observed.txt").unwrap();
        inner.write_to(fh, 0, b"witness me").unwrap();
        assert!(inner.read_annotations().is_empty());

        let _data = inner.read_file(ino, 0, 1024).unwrap();

        assert_eq!(inner.read_annotations().len(), 1);
        let ann = &inner.read_annotations()[0];
        assert_eq!(ann.path, "observed.txt");
        assert_eq!(ann.visibility, "public");
        assert_eq!(
            ann.content_hash,
            fragmentation::fragment::blob_oid_bytes(b"witness me"),
        );
        assert!(!ann.timestamp.is_empty());
    }

    #[test]
    fn read_file_private_path_annotated_as_private() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let dir_ino = inner.mkdir(1, "private").unwrap();
        let (ino, fh) = inner.create_file(dir_ino, "secret.key").unwrap();
        inner.write_to(fh, 0, b"key material").unwrap();

        let _data = inner.read_file(ino, 0, 1024).unwrap();

        assert_eq!(inner.read_annotations()[0].visibility, "private");
        assert_eq!(inner.read_annotations()[0].path, "private/secret.key");
    }

    #[test]
    fn read_file_protected_path_annotated_as_protected() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let dir_ino = inner.mkdir(1, "protected").unwrap();
        let (ino, fh) = inner.create_file(dir_ino, "draft.md").unwrap();
        inner.write_to(fh, 0, b"draft content").unwrap();

        let _data = inner.read_file(ino, 0, 1024).unwrap();

        assert_eq!(inner.read_annotations()[0].visibility, "protected");
    }

    #[test]
    fn read_file_offset_beyond_end_no_annotation() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "short.txt").unwrap();
        inner.write_to(fh, 0, b"hi").unwrap();

        // Read past end returns empty — no annotation for empty read.
        let data = inner.read_file(ino, 100, 1024).unwrap();
        assert!(data.is_empty());
        assert!(
            inner.read_annotations().is_empty(),
            "offset-past-end should not create an annotation",
        );
    }

    #[test]
    fn multiple_reads_accumulate_annotations() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "multi.txt").unwrap();
        inner.write_to(fh, 0, b"data").unwrap();

        inner.read_file(ino, 0, 1024).unwrap();
        inner.read_file(ino, 0, 1024).unwrap();
        inner.read_file(ino, 0, 1024).unwrap();

        assert_eq!(inner.read_annotations().len(), 3);
    }

    // -----------------------------------------------------------------------
    // @read annotation — flush commits annotations
    // -----------------------------------------------------------------------

    #[test]
    fn flush_commits_read_annotations() {
        let (dir, mut inner) = make_inner("refs/fragmentation/test");

        // Write a file and flush to establish ref
        let (ino, fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(fh, 0, b"content").unwrap();
        inner.flush(fh, "fuse: file.txt").unwrap();
        inner.release(fh);
        let head_after_write = inner.head().unwrap();

        // Read the file — creates annotation
        inner.read_file(ino, 0, 1024).unwrap();
        assert_eq!(inner.read_annotations().len(), 1);

        // Flush a clean handle — should still commit due to pending annotation
        let fh2 = inner.open_existing(ino);
        inner.flush(fh2, "fuse: read").unwrap();

        let head_after_read = inner.head().unwrap();
        assert_ne!(
            head_after_write, head_after_read,
            "read annotation should create a new commit"
        );

        // Verify @read tree is in the commit
        let repo = git2::Repository::open(dir.path()).unwrap();
        let commit = repo.find_commit(head_after_read).unwrap();
        let tree = commit.tree().unwrap();
        let read_entry = tree.get_name("@read");
        assert!(
            read_entry.is_some(),
            "@read subtree should exist in committed tree"
        );
    }

    #[test]
    fn flush_clears_annotations_after_commit() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (ino, fh) = inner.create_file(1, "file.txt").unwrap();
        inner.write_to(fh, 0, b"content").unwrap();

        inner.read_file(ino, 0, 1024).unwrap();
        assert_eq!(inner.read_annotations().len(), 1);

        inner.flush(fh, "fuse: file.txt").unwrap();
        assert!(
            inner.read_annotations().is_empty(),
            "annotations should be cleared after flush",
        );
    }

    #[test]
    fn flush_noop_when_clean_and_no_annotations() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (_ino, fh) = inner.create_file(1, "a.txt").unwrap();
        // No write, no read — should be noop
        inner.flush(fh, "fuse: a.txt").unwrap();
        assert!(
            inner.head().is_none(),
            "clean flush with no annotations should not commit"
        );
    }

    // -----------------------------------------------------------------------
    // Lens — helper
    // -----------------------------------------------------------------------

    fn make_lens_inner(ref_name: &str) -> (tempfile::TempDir, FsInner) {
        use fragmentation::fragment::Fractal;
        use fragmentation::git::write_tree_named;
        use fragmentation::ref_::Ref;
        use fragmentation::sha::Sha;

        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_owned();

        {
            let repo = git2::Repository::init(&repo_path).unwrap();

            // Target tree with a file
            let target_file = Fractal::<Vec<u8>>::shard_typed(
                Ref::new(Sha("f".to_string()), "greeting.txt"),
                b"hello from target".to_vec(),
            );
            let target_tree = Fractal::<Vec<u8>>::new_typed(
                Ref::new(Sha("t".to_string()), "target"),
                b"".to_vec(),
                vec![target_file],
            );
            let target_oid = write_tree_named(&repo, &target_tree).unwrap();

            // Root tree with a lens pointing to target
            let lens = Fractal::<Vec<u8>>::lens_typed(
                Ref::new(Sha("l".to_string()), "my-lens"),
                b"".to_vec(),
                vec![Sha(target_oid.to_string())],
            );
            let root = Fractal::<Vec<u8>>::new_typed(
                Ref::new(Sha("r".to_string()), "/"),
                b"".to_vec(),
                vec![lens],
            );
            let root_oid = write_tree_named(&repo, &root).unwrap();

            let sig = git2::Signature::now("test", "test@test").unwrap();
            let tree = repo.find_tree(root_oid).unwrap();
            repo.commit(Some(ref_name), &sig, &sig, "lens commit", &tree, &[])
                .unwrap();
        }

        let repo = git2::Repository::open(&repo_path).unwrap();
        let committer = Committer::new("test", "test@test");
        let inner = FsInner::from_ref(repo, committer, ref_name.to_string()).unwrap();

        (dir, inner)
    }

    // -----------------------------------------------------------------------
    // Lens — populate + inspect (cycle 5)
    // -----------------------------------------------------------------------

    #[test]
    fn populate_lens_as_directory() {
        let (_dir, inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner
            .lookup_child(1, "my-lens")
            .expect("my-lens should exist");
        assert!(inner.is_dir(lens_ino), "lens should appear as directory");
    }

    #[test]
    fn lens_is_dir() {
        let (_dir, inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner.lookup_child(1, "my-lens").unwrap();
        assert!(inner.is_dir(lens_ino));
        assert!(inner.is_lens(lens_ino));
    }

    #[test]
    fn lens_lookup_child() {
        let (_dir, inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner.lookup_child(1, "my-lens").unwrap();
        let file_ino = inner.lookup_child(lens_ino, "greeting.txt");
        assert!(file_ino.is_some(), "greeting.txt should be inside lens");
    }

    #[test]
    fn build_fractal_from_lens_node() {
        let (dir, mut inner) = make_lens_inner("refs/fragmentation/test");
        // Flush with a write to trigger commit
        let (_ino, fh) = inner.create_file(1, "regular.txt").unwrap();
        inner.write_to(fh, 0, b"regular content").unwrap();
        inner.flush(fh, "fuse: mixed").unwrap();

        // Verify lens is preserved in committed tree
        let repo = git2::Repository::open(dir.path()).unwrap();
        let commit = repo.find_commit(inner.head().unwrap()).unwrap();
        let tree_oid = commit.tree_id();
        let fractal = fragmentation::git::read_tree_named(&repo, tree_oid).unwrap();

        use fragmentation::fragment::Fragmentable;
        let lens_child = fractal
            .children()
            .iter()
            .find(|c| c.self_ref().label == "my-lens");
        assert!(lens_child.is_some(), "my-lens should be in committed tree");
        assert!(lens_child.unwrap().is_lens());
    }

    // -----------------------------------------------------------------------
    // Lens — read-only enforcement (cycle 6)
    // -----------------------------------------------------------------------

    #[test]
    fn create_file_in_lens_returns_read_only() {
        let (_dir, mut inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner.lookup_child(1, "my-lens").unwrap();
        let result = inner.create_file(lens_ino, "new-file.txt");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::ReadOnly));
    }

    #[test]
    fn mkdir_in_lens_returns_read_only() {
        let (_dir, mut inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner.lookup_child(1, "my-lens").unwrap();
        let result = inner.mkdir(lens_ino, "new-dir");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::ReadOnly));
    }

    #[test]
    fn write_under_lens_returns_read_only() {
        let (_dir, mut inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner.lookup_child(1, "my-lens").unwrap();
        let file_ino = inner.lookup_child(lens_ino, "greeting.txt").unwrap();
        let fh = inner.open_existing(file_ino);
        let result = inner.write_to(fh, 0, b"attempt write");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::ReadOnly));
    }

    #[test]
    fn unlink_in_lens_returns_read_only() {
        let (_dir, mut inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner.lookup_child(1, "my-lens").unwrap();
        let result = inner.unlink(lens_ino, "greeting.txt");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::ReadOnly));
    }

    #[test]
    fn is_dir_false_for_file_and_missing_inode() {
        let (_dir, mut inner) = make_inner("refs/fragmentation/test");
        let (file_ino, _fh) = inner.create_file(1, "file.txt").unwrap();
        assert!(!inner.is_dir(file_ino), "file inode should not be a dir");
        assert!(!inner.is_dir(99999), "non-existent inode should not be a dir");
    }

    #[test]
    fn rmdir_under_lens_returns_read_only() {
        let (_dir, mut inner) = make_lens_inner("refs/fragmentation/test");
        let lens_ino = inner.lookup_child(1, "my-lens").unwrap();
        let result = inner.rmdir(lens_ino, "greeting.txt");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::ReadOnly));
    }

    #[test]
    fn fs_error_display_read_only() {
        assert_eq!(format!("{}", FsError::ReadOnly), "read-only filesystem");
    }

    // -----------------------------------------------------------------------
    // @read annotation — flush commits annotations
    // -----------------------------------------------------------------------

    #[test]
    fn flush_read_annotation_contains_serialized_data() {
        let (dir, mut inner) = make_inner("refs/fragmentation/test");

        let (ino, fh) = inner.create_file(1, "witness.txt").unwrap();
        inner.write_to(fh, 0, b"observed").unwrap();

        // Read creates annotation
        inner.read_file(ino, 0, 1024).unwrap();

        // Flush commits everything (dirty from write + annotation from read)
        inner.flush(fh, "fuse: witness.txt").unwrap();

        // Verify the @read subtree has annotation data
        let repo = git2::Repository::open(dir.path()).unwrap();
        let commit = repo.find_commit(inner.head().unwrap()).unwrap();
        let tree = commit.tree().unwrap();
        let read_entry = tree.get_name("@read").unwrap();
        let read_tree = repo.find_tree(read_entry.id()).unwrap();

        // Should have .data and one annotation shard (0000)
        let ann_entry = read_tree.get_name("0000");
        assert!(
            ann_entry.is_some(),
            "annotation shard 0000 should exist under @read"
        );

        let blob = repo.find_blob(ann_entry.unwrap().id()).unwrap();
        let content = std::str::from_utf8(blob.content()).unwrap();
        assert!(
            content.contains("path=witness.txt"),
            "annotation should contain path"
        );
        assert!(
            content.contains("visibility=public"),
            "annotation should contain visibility"
        );
        assert!(
            content.contains(&format!(
                "content_hash={}",
                fragmentation::fragment::blob_oid_bytes(b"observed")
            )),
            "annotation should contain content_hash",
        );
        assert!(
            content.contains("timestamp="),
            "annotation should contain timestamp"
        );
    }
}
