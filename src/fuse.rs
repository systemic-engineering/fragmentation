//! FUSE filesystem backed by fragmentation.
//!
//! Every `flush()` on a written file creates a git commit at
//! `refs/<namespace>/<ref_name>`. The filesystem state lives in an inode
//! table; each flush snapshots the whole tree as a `Fractal<Vec<u8>>` and
//! writes it through `write_tree_named`.
//!
//! Reads are witnessed too. Every `read_file()` call creates a `ReadAnnotation`
//! recording the path, visibility tier, and content hash of the data served.
//! Pending annotations are committed on the next `flush()` as `@read` shards
//! alongside the filesystem tree. An actor reading identity files through the
//! mount produces a witnessed trace automatically — the read cannot be un-read.
//!
//! `refs/heads/main` is never touched. The fragmentation commit chain lives
//! in its own namespace coexisting with regular git history.
//!
//! Two feature levels:
//! - `fuse`       — FsInner state machine (no FUSE library dependency).
//! - `fuse-mount` — FragmentFs + fuser::Filesystem impl (requires macFUSE).

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};

use crate::fragment::{Fractal, Fragmentable};
use crate::git::{read_tree_named, write_tree_named};
use crate::ref_::Ref;
use crate::sha::Sha;
use crate::witnessed::Committer;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum FsError {
    BadFd,
    NotFound,
    NotAFile,
    NotADir,
    NotEmpty,
    Git(git2::Error),
    Other(String),
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsError::BadFd => write!(f, "bad file descriptor"),
            FsError::NotFound => write!(f, "not found"),
            FsError::NotAFile => write!(f, "not a file"),
            FsError::NotADir => write!(f, "not a directory"),
            FsError::NotEmpty => write!(f, "directory not empty"),
            FsError::Git(e) => write!(f, "git error: {}", e),
            FsError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for FsError {}

impl From<git2::Error> for FsError {
    fn from(e: git2::Error) -> Self {
        FsError::Git(e)
    }
}

// ---------------------------------------------------------------------------
// Inode table
// ---------------------------------------------------------------------------

type Ino = u64;
type Fh = u64;

enum Node {
    Dir { children: HashMap<OsString, Ino> },
    File { content: Vec<u8> },
}

struct OpenFileMeta {
    ino: Ino,
    dirty: bool,
}

// ---------------------------------------------------------------------------
// @read annotation — witnessed reads
// ---------------------------------------------------------------------------

/// A record of a file read through the FUSE mount.
///
/// Created eagerly in `read_file()`, committed on the next `flush()`.
/// The annotation cannot be un-read: once the bytes are served, the
/// record exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadAnnotation {
    /// Path relative to mount root (e.g. "private/keys/id.pub").
    pub path: String,
    /// Visibility tier derived from path prefix: "private", "protected", or "public".
    pub visibility: &'static str,
    /// SHA of the content that was served (blob OID of the full file content).
    pub content_hash: String,
    /// When the read occurred (opaque string, typically seconds since epoch).
    pub timestamp: String,
}

/// Derive visibility tier from a path relative to the mount root.
///
/// Paths under `private/` are private. Paths under `protected/` are protected.
/// Everything else is public. Leading slashes are tolerated.
pub fn path_visibility(path: &str) -> &'static str {
    let p = path.strip_prefix('/').unwrap_or(path);
    if p.starts_with("private/") {
        "private"
    } else if p.starts_with("protected/") {
        "protected"
    } else {
        "public"
    }
}

// ---------------------------------------------------------------------------
// FsInner — the actual filesystem state (no FUSE dependency)
// ---------------------------------------------------------------------------

pub struct FsInner {
    repo: git2::Repository,
    committer: Committer,
    inodes: HashMap<Ino, Node>,
    parents: HashMap<Ino, (Ino, OsString)>,
    next_ino: Ino,
    open_files: HashMap<Fh, OpenFileMeta>,
    next_fh: Fh,
    head: Option<git2::Oid>,
    ref_name: String,
    read_annotations: Vec<ReadAnnotation>,
}

impl FsInner {
    /// Create a fresh filesystem with an empty root directory (ino=1).
    pub fn new(repo: git2::Repository, committer: Committer, ref_name: String) -> Self {
        let mut inodes = HashMap::new();
        inodes.insert(
            1,
            Node::Dir {
                children: HashMap::new(),
            },
        );

        FsInner {
            repo,
            committer,
            inodes,
            parents: HashMap::new(),
            next_ino: 2,
            open_files: HashMap::new(),
            next_fh: 1,
            head: None,
            ref_name,
            read_annotations: Vec::new(),
        }
    }

    /// Load filesystem state from an existing fragmentation ref.
    pub fn from_ref(
        repo: git2::Repository,
        committer: Committer,
        ref_name: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Extract what we need before consuming repo. All borrows from repo
        // (ref_obj, commit, tree) are dropped at the end of this block.
        let (commit_oid, fractal) = {
            let ref_obj = repo.find_reference(&ref_name)?;
            let commit_oid = ref_obj.target().ok_or("ref is not a direct reference")?;
            let commit = repo.find_commit(commit_oid)?;
            let tree_oid = commit.tree_id();
            let fractal = read_tree_named(&repo, tree_oid)?;
            (commit_oid, fractal)
        };

        let mut inner = FsInner {
            repo,
            committer,
            inodes: HashMap::new(),
            parents: HashMap::new(),
            next_ino: 2,
            open_files: HashMap::new(),
            next_fh: 1,
            head: Some(commit_oid),
            ref_name,
            read_annotations: Vec::new(),
        };

        inner.inodes.insert(
            1,
            Node::Dir {
                children: HashMap::new(),
            },
        );
        inner.populate_from_fractal(&fractal, 1)?;

        Ok(inner)
    }

    fn populate_from_fractal(
        &mut self,
        fractal: &Fractal<Vec<u8>>,
        parent_ino: Ino,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for child in fractal.children() {
            let name = OsString::from(&child.self_ref().label);
            let ino = self.next_ino;
            self.next_ino += 1;

            if child.is_shard() {
                self.inodes.insert(
                    ino,
                    Node::File {
                        content: child.data().clone(),
                    },
                );
            } else {
                self.inodes.insert(
                    ino,
                    Node::Dir {
                        children: HashMap::new(),
                    },
                );
                self.populate_from_fractal(child, ino)?;
            }

            self.parents.insert(ino, (parent_ino, name.clone()));
            if let Some(Node::Dir { children }) = self.inodes.get_mut(&parent_ino) {
                children.insert(name, ino);
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public inspection API (for tests)
    // -----------------------------------------------------------------------

    pub fn has_inode(&self, ino: Ino) -> bool {
        self.inodes.contains_key(&ino)
    }

    pub fn is_dir(&self, ino: Ino) -> bool {
        matches!(self.inodes.get(&ino), Some(Node::Dir { .. }))
    }

    pub fn head(&self) -> Option<git2::Oid> {
        self.head
    }

    pub fn parent_of_head(&self) -> Option<git2::Oid> {
        let head_oid = self.head?;
        let commit = self.repo.find_commit(head_oid).ok()?;
        commit.parent_id(0).ok()
    }

    /// Pending read annotations accumulated since last flush.
    pub fn read_annotations(&self) -> &[ReadAnnotation] {
        &self.read_annotations
    }

    /// Reconstruct the path of an inode by walking the parents map to root.
    /// Returns a path relative to the mount root, e.g. "private/keys/id.pub".
    /// Returns None if the inode has no parents entry (root directory).
    pub fn ino_path(&self, ino: Ino) -> Option<String> {
        let mut segments = Vec::new();
        let mut current = ino;
        while let Some((parent, name)) = self.parents.get(&current) {
            segments.push(name.to_str().unwrap_or("").to_string());
            current = *parent;
        }
        if segments.is_empty() {
            return None;
        }
        segments.reverse();
        Some(segments.join("/"))
    }

    pub fn lookup_child(&self, parent_ino: Ino, name: &str) -> Option<Ino> {
        if let Some(Node::Dir { children }) = self.inodes.get(&parent_ino) {
            children.get(OsStr::new(name)).copied()
        } else {
            None
        }
    }

    pub fn read_buffer(&self, fh: Fh) -> &[u8] {
        let meta = match self.open_files.get(&fh) {
            Some(m) => m,
            None => return &[],
        };
        match self.inodes.get(&meta.ino) {
            Some(Node::File { content }) => content.as_slice(),
            _ => &[],
        }
    }

    // -----------------------------------------------------------------------
    // Filesystem mutation API
    // -----------------------------------------------------------------------

    /// Create a new file in parent_ino. Returns (ino, fh).
    pub fn create_file(&mut self, parent_ino: Ino, name: &str) -> Result<(Ino, Fh), FsError> {
        if !matches!(self.inodes.get(&parent_ino), Some(Node::Dir { .. })) {
            return Err(FsError::NotADir);
        }
        let ino = self.next_ino;
        self.next_ino += 1;
        self.inodes.insert(
            ino,
            Node::File {
                content: Vec::new(),
            },
        );
        self.parents.insert(ino, (parent_ino, OsString::from(name)));
        if let Some(Node::Dir { children }) = self.inodes.get_mut(&parent_ino) {
            children.insert(OsString::from(name), ino);
        }
        let fh = self.alloc_fh(ino);
        Ok((ino, fh))
    }

    /// Open an existing file by inode. Returns a new file handle.
    pub fn open_existing(&mut self, ino: Ino) -> Fh {
        self.alloc_fh(ino)
    }

    /// Write data to an open file handle at offset.
    pub fn write_to(&mut self, fh: Fh, offset: i64, data: &[u8]) -> Result<(), FsError> {
        let meta = self.open_files.get_mut(&fh).ok_or(FsError::BadFd)?;
        let ino = meta.ino;
        meta.dirty = true;

        let offset = offset as usize;
        let content = match self.inodes.get_mut(&ino) {
            Some(Node::File { content }) => content,
            _ => return Err(FsError::NotAFile),
        };
        if content.len() < offset {
            content.resize(offset, 0);
        }
        if content.len() < offset + data.len() {
            content.resize(offset + data.len(), 0);
        }
        content[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Read from a file inode. Returns bytes from offset up to size.
    ///
    /// Also creates a `ReadAnnotation` recording the path, visibility, and
    /// content hash of the file that was read. The annotation is accumulated
    /// and committed on the next `flush()`.
    pub fn read_file(&mut self, ino: Ino, offset: i64, size: u32) -> Result<Vec<u8>, FsError> {
        match self.inodes.get(&ino) {
            Some(Node::File { content }) => {
                let offset = offset as usize;
                if offset >= content.len() {
                    return Ok(Vec::new());
                }
                let end = std::cmp::min(offset + size as usize, content.len());
                let data = content[offset..end].to_vec();

                // Witness the read: eager annotation before the response.
                let content_hash = crate::fragment::blob_oid_bytes(content);
                if let Some(path) = self.ino_path(ino) {
                    let visibility = path_visibility(&path);
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs().to_string())
                        .unwrap_or_else(|_| "0".to_string());
                    self.read_annotations.push(ReadAnnotation {
                        path,
                        visibility,
                        content_hash,
                        timestamp,
                    });
                }

                Ok(data)
            }
            _ => Err(FsError::NotAFile),
        }
    }

    /// Commit the current filesystem snapshot if the file handle is dirty
    /// or there are pending read annotations.
    ///
    /// Builds a complete Fractal from the inode table. If there are pending
    /// `@read` annotations, they are included as shards under a `@read`
    /// subtree alongside the filesystem content.
    pub fn flush(&mut self, fh: Fh, message: &str) -> Result<(), FsError> {
        let dirty = self.open_files.get(&fh).map(|m| m.dirty).unwrap_or(false);
        let has_annotations = !self.read_annotations.is_empty();

        if !dirty && !has_annotations {
            return Ok(());
        }

        let mut root_fractal = self.build_fractal_for_ino(1, "/");

        // Attach @read annotations as shards under a @read subtree.
        if has_annotations {
            let annotation_shards: Vec<Fractal<Vec<u8>>> = self
                .read_annotations
                .iter()
                .enumerate()
                .map(|(i, ann)| {
                    let serialized = format!(
                        "path={}\nvisibility={}\ncontent_hash={}\ntimestamp={}",
                        ann.path, ann.visibility, ann.content_hash, ann.timestamp,
                    );
                    let bytes = serialized.into_bytes();
                    let sha = crate::fragment::blob_oid_bytes(&bytes);
                    let label = format!("{:04}", i);
                    let ref_ = Ref::new(Sha(sha), label);
                    Fractal::shard_typed(ref_, bytes)
                })
                .collect();

            let read_ref = Ref::new(Sha("0".to_string()), "@read");
            let read_fractal = Fractal::new_typed(read_ref, vec![], annotation_shards);

            // Inject @read subtree into root fractal's children.
            if let Fractal::Fractal { fractal, .. } = &mut root_fractal {
                fractal.push(read_fractal);
            }

            self.read_annotations.clear();
        }

        let commit_msg = if dirty && has_annotations {
            message.to_string()
        } else if has_annotations {
            "fuse: @read".to_string()
        } else {
            message.to_string()
        };

        let commit_oid = self.write_commit_internal(&root_fractal, &commit_msg)?;

        self.head = Some(commit_oid);
        if let Some(meta) = self.open_files.get_mut(&fh) {
            meta.dirty = false;
        }
        Ok(())
    }

    /// Release a file handle.
    pub fn release(&mut self, fh: Fh) {
        self.open_files.remove(&fh);
    }

    /// Create a directory in parent_ino. Returns new ino.
    pub fn mkdir(&mut self, parent_ino: Ino, name: &str) -> Result<Ino, FsError> {
        if !matches!(self.inodes.get(&parent_ino), Some(Node::Dir { .. })) {
            return Err(FsError::NotADir);
        }
        let ino = self.next_ino;
        self.next_ino += 1;
        self.inodes.insert(
            ino,
            Node::Dir {
                children: HashMap::new(),
            },
        );
        self.parents.insert(ino, (parent_ino, OsString::from(name)));
        if let Some(Node::Dir { children }) = self.inodes.get_mut(&parent_ino) {
            children.insert(OsString::from(name), ino);
        }
        Ok(ino)
    }

    /// Remove a file from its parent directory.
    pub fn unlink(&mut self, parent_ino: Ino, name: &str) -> Result<(), FsError> {
        let ino = {
            match self.inodes.get(&parent_ino) {
                Some(Node::Dir { children }) => {
                    *children.get(OsStr::new(name)).ok_or(FsError::NotFound)?
                }
                _ => return Err(FsError::NotADir),
            }
        };
        self.inodes.remove(&ino);
        self.parents.remove(&ino);
        if let Some(Node::Dir { children }) = self.inodes.get_mut(&parent_ino) {
            children.remove(OsStr::new(name));
        }
        Ok(())
    }

    /// Remove an empty directory from its parent.
    pub fn rmdir(&mut self, parent_ino: Ino, name: &str) -> Result<(), FsError> {
        let ino = {
            match self.inodes.get(&parent_ino) {
                Some(Node::Dir { children }) => {
                    *children.get(OsStr::new(name)).ok_or(FsError::NotFound)?
                }
                _ => return Err(FsError::NotADir),
            }
        };
        if let Some(Node::Dir { children }) = self.inodes.get(&ino) {
            if !children.is_empty() {
                return Err(FsError::NotEmpty);
            }
        }
        self.inodes.remove(&ino);
        self.parents.remove(&ino);
        if let Some(Node::Dir { children }) = self.inodes.get_mut(&parent_ino) {
            children.remove(OsStr::new(name));
        }
        Ok(())
    }

    /// Truncate a file to the given size.
    pub fn truncate(&mut self, ino: Ino, size: usize) {
        if let Some(Node::File { content }) = self.inodes.get_mut(&ino) {
            if content.len() > size {
                content.truncate(size);
            } else {
                content.resize(size, 0);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn alloc_fh(&mut self, ino: Ino) -> Fh {
        let fh = self.next_fh;
        self.next_fh += 1;
        self.open_files
            .insert(fh, OpenFileMeta { ino, dirty: false });
        fh
    }

    /// Recursively build a Fractal<Vec<u8>> from the inode table.
    fn build_fractal_for_ino(&self, ino: Ino, name: &str) -> Fractal<Vec<u8>> {
        match self.inodes.get(&ino) {
            Some(Node::File { content }) => {
                let sha = crate::fragment::blob_oid_bytes(content);
                let ref_ = Ref::new(Sha(sha), name);
                Fractal::shard_typed(ref_, content.clone())
            }
            Some(Node::Dir { children }) => {
                let child_fractals: Vec<Fractal<Vec<u8>>> = children
                    .iter()
                    .map(|(child_name, &child_ino)| {
                        self.build_fractal_for_ino(child_ino, child_name.to_str().unwrap_or(""))
                    })
                    .collect();
                let ref_ = Ref::new(Sha("0".to_string()), name);
                Fractal::new_typed(ref_, vec![], child_fractals)
            }
            None => panic!("inode {} not found", ino),
        }
    }

    /// Write the fractal as a git commit to self.ref_name. Uses &self (no mutation).
    fn write_commit_internal(
        &self,
        fractal: &Fractal<Vec<u8>>,
        message: &str,
    ) -> Result<git2::Oid, FsError> {
        let tree_oid = write_tree_named(&self.repo, fractal)?;
        let tree = self.repo.find_tree(tree_oid)?;
        let sig = git2::Signature::now(&self.committer.name, &self.committer.email)
            .map_err(FsError::Git)?;

        let commit_oid = if let Some(parent_oid) = self.head {
            let parent_commit = self.repo.find_commit(parent_oid)?;
            self.repo.commit(
                Some(&self.ref_name),
                &sig,
                &sig,
                message,
                &tree,
                &[&parent_commit],
            )?
        } else {
            self.repo
                .commit(Some(&self.ref_name), &sig, &sig, message, &tree, &[])?
        };

        Ok(commit_oid)
    }
}

// ---------------------------------------------------------------------------
// FragmentFs — FUSE filesystem wrapper (requires macFUSE at runtime)
// ---------------------------------------------------------------------------

#[cfg(feature = "fuse-mount")]
pub struct FragmentFs {
    inner: std::sync::Mutex<FsInner>,
}

#[cfg(feature = "fuse-mount")]
impl FragmentFs {
    /// Open a filesystem backed by the given repo and ref.
    /// If the ref exists, loads existing state. Otherwise starts fresh.
    pub fn open(repo: git2::Repository, committer: Committer, full_ref: String) -> Self {
        let inner = if repo.find_reference(&full_ref).is_ok() {
            FsInner::from_ref(repo, committer, full_ref)
                .unwrap_or_else(|e| panic!("failed to load from ref: {}", e))
        } else {
            FsInner::new(repo, committer, full_ref)
        };
        FragmentFs {
            inner: std::sync::Mutex::new(inner),
        }
    }

    fn make_attr(inner: &FsInner, ino: Ino) -> fuser::FileAttr {
        let now = std::time::SystemTime::now();
        match inner.inodes.get(&ino) {
            Some(Node::File { content }) => fuser::FileAttr {
                ino,
                size: content.len() as u64,
                blocks: (content.len() as u64).div_ceil(512),
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: fuser::FileType::RegularFile,
                perm: 0o644,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                blksize: 512,
                flags: 0,
            },
            Some(Node::Dir { children }) => fuser::FileAttr {
                ino,
                size: 0,
                blocks: 0,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: fuser::FileType::Directory,
                perm: 0o755,
                nlink: 2 + children.len() as u32,
                uid: 0,
                gid: 0,
                rdev: 0,
                blksize: 512,
                flags: 0,
            },
            None => panic!("inode {} not found", ino),
        }
    }

    fn readdir_entries(inner: &FsInner, ino: Ino) -> Vec<(Ino, fuser::FileType, OsString)> {
        let parent_ino = inner.parents.get(&ino).map(|(p, _)| *p).unwrap_or(ino);

        let mut entries = vec![
            (ino, fuser::FileType::Directory, OsString::from(".")),
            (parent_ino, fuser::FileType::Directory, OsString::from("..")),
        ];

        if let Some(Node::Dir { children }) = inner.inodes.get(&ino) {
            for (name, &child_ino) in children {
                let kind = match inner.inodes.get(&child_ino) {
                    Some(Node::Dir { .. }) => fuser::FileType::Directory,
                    Some(Node::File { .. }) => fuser::FileType::RegularFile,
                    None => continue,
                };
                entries.push((child_ino, kind, name.clone()));
            }
        }
        entries
    }
}

// ---------------------------------------------------------------------------
// fuser::Filesystem impl
// ---------------------------------------------------------------------------

#[cfg(feature = "fuse-mount")]
impl fuser::Filesystem for FragmentFs {
    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let inner = self.inner.lock().unwrap();
        match inner.lookup_child(parent, name.to_str().unwrap_or("")) {
            Some(ino) => {
                let attr = Self::make_attr(&inner, ino);
                reply.entry(&std::time::Duration::ZERO, &attr, 0);
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn getattr(&mut self, _req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyAttr) {
        let inner = self.inner.lock().unwrap();
        if inner.has_inode(ino) {
            let attr = Self::make_attr(&inner, ino);
            reply.attr(&std::time::Duration::ZERO, &attr);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn setattr(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
        reply: fuser::ReplyAttr,
    ) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(sz) = size {
            inner.truncate(ino, sz as usize);
        }
        if inner.has_inode(ino) {
            let attr = Self::make_attr(&inner, ino);
            reply.attr(&std::time::Duration::ZERO, &attr);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        let inner = self.inner.lock().unwrap();
        let entries = Self::readdir_entries(&inner, ino);
        for (i, (child_ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(child_ino, (i + 1) as i64, kind, &name) {
                break;
            }
        }
        reply.ok();
    }

    fn create(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        let mut inner = self.inner.lock().unwrap();
        match inner.create_file(parent, name.to_str().unwrap_or("")) {
            Ok((ino, fh)) => {
                let attr = Self::make_attr(&inner, ino);
                reply.created(&std::time::Duration::ZERO, &attr, 0, fh, 0);
            }
            Err(_) => reply.error(libc::EIO),
        }
    }

    fn open(&mut self, _req: &fuser::Request<'_>, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        let mut inner = self.inner.lock().unwrap();
        let fh = inner.open_existing(ino);
        reply.opened(fh, 0);
    }

    fn read(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        let mut inner = self.inner.lock().unwrap();
        match inner.read_file(ino, offset, size) {
            Ok(data) => reply.data(&data),
            Err(_) => reply.error(libc::EIO),
        }
    }

    fn write(
        &mut self,
        _req: &fuser::Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let n = data.len() as u32;
        match inner.write_to(fh, offset, data) {
            Ok(()) => reply.written(n),
            Err(_) => reply.error(libc::EIO),
        }
    }

    fn flush(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        _lock_owner: u64,
        reply: fuser::ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let message = format!("fuse: write ino={}", ino);
        match inner.flush(fh, &message) {
            Ok(()) => reply.ok(),
            Err(e) => {
                eprintln!("flush error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    fn release(
        &mut self,
        _req: &fuser::Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().unwrap();
        inner.release(fh);
        reply.ok();
    }

    fn mkdir(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: fuser::ReplyEntry,
    ) {
        let mut inner = self.inner.lock().unwrap();
        match inner.mkdir(parent, name.to_str().unwrap_or("")) {
            Ok(ino) => {
                let attr = Self::make_attr(&inner, ino);
                reply.entry(&std::time::Duration::ZERO, &attr, 0);
            }
            Err(_) => reply.error(libc::EIO),
        }
    }

    fn rmdir(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().unwrap();
        match inner.rmdir(parent, name.to_str().unwrap_or("")) {
            Ok(()) => reply.ok(),
            Err(_) => reply.error(libc::EIO),
        }
    }

    fn unlink(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().unwrap();
        match inner.unlink(parent, name.to_str().unwrap_or("")) {
            Ok(()) => reply.ok(),
            Err(_) => reply.error(libc::EIO),
        }
    }
}
