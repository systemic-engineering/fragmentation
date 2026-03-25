#[cfg(feature = "git")]
use crate::encoding::{Decode, Encode};

#[cfg(feature = "git")]
use crate::fragment::{Fractal, Fragmentable, Reconstructable};

#[cfg(feature = "git")]
use crate::witnessed::Witnessed;

/// Read witness metadata from any git commit.
/// Returns (Witnessed, Message, tree OID). Works on any commit, not just fragmentation ones.
#[cfg(feature = "git")]
pub fn read_witnessed(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<(Witnessed, crate::witnessed::Message, git2::Oid), Box<dyn std::error::Error>> {
    use crate::witnessed::{Author, Committer, Message, Timestamp};

    let commit = repo.find_commit(oid)?;
    let author = Author::new(
        commit.author().name().unwrap_or(""),
        commit.author().email().unwrap_or(""),
    );
    let committer = Committer::new(
        commit.committer().name().unwrap_or(""),
        commit.committer().email().unwrap_or(""),
    );
    let timestamp = Timestamp(commit.time().seconds().to_string());
    let message = Message(commit.message().unwrap_or("").to_string());
    let witnessed = Witnessed::new(author, committer, timestamp);
    Ok((witnessed, message, commit.tree_id()))
}

/// Read a fragmentation commit. Returns Commit<Fractal<String>> (Root or Child) with full metadata.
/// Only works on commits written by write_commit (fragmentation-format trees).
#[cfg(feature = "git")]
pub fn read_commit(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<crate::commit::Commit<Fractal<String>>, Box<dyn std::error::Error>> {
    use crate::commit::Parent;
    use crate::sha::Sha;

    let git_commit = repo.find_commit(oid)?;
    let (witnessed, message, tree_oid) = read_witnessed(repo, oid)?;
    let fractal = read_tree(repo, tree_oid)?;
    let sha = Sha(oid.to_string());

    match git_commit.parent_id(0).ok() {
        None => Ok(crate::commit::Commit::full_root(
            fractal, witnessed, message, sha,
        )),
        Some(parent_oid) => Ok(crate::commit::Commit::full_child(
            fractal,
            witnessed,
            message,
            Parent(Sha(parent_oid.to_string())),
            sha,
        )),
    }
}

/// Extract the signature from a signed commit, if present.
/// Returns None for unsigned commits.
#[cfg(feature = "git")]
pub fn commit_signature(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    match repo.extract_signature(&oid, None) {
        Ok((sig, _signed_data)) => Ok(Some(sig.to_vec())),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Write a fragment tree to git objects. Returns the root OID.
/// Shard -> blob, Fractal -> tree with .data + numbered children.
#[cfg(feature = "git")]
pub fn write_tree<E: Encode>(
    repo: &git2::Repository,
    fragment: &Fractal<E>,
) -> Result<git2::Oid, git2::Error> {
    use crate::sha::HashAlg;

    match fragment {
        Fractal::Shard { data, .. } => repo.blob(&data.encode()),
        Fractal::Fractal { data, fractal, .. } => {
            let mut builder = repo.treebuilder(None)?;

            let data_oid = repo.blob(&data.encode())?;
            builder.insert(".data", data_oid, 0o100644)?;

            for (i, child) in fractal.iter().enumerate() {
                let child_oid = write_tree(repo, child)?;
                let mode = if child.is_shard() { 0o100644 } else { 0o040000 };
                builder.insert(format!("{:04}", i), child_oid, mode)?;
            }

            builder.write()
        }
        Fractal::Lens { data, target, .. } => {
            let mut builder = repo.treebuilder(None)?;

            let data_oid = repo.blob(&data.encode())?;
            builder.insert(".data", data_oid, 0o100644)?;

            let lens_content: String = target
                .iter()
                .map(|sha| sha.as_str())
                .collect::<Vec<&str>>()
                .join("\n");
            let lens_oid = repo.blob(lens_content.as_bytes())?;
            builder.insert(".lens", lens_oid, 0o100644)?;

            builder.write()
        }
    }
}

/// Write any Fragmentable to the git object database.
///
/// Shard (no children) → git blob.
/// Node with children → git tree (.data blob + numbered child entries).
///
/// This is the generic version of `write_tree` — works with any
/// Fragmentable, not just Fractal.
#[cfg(feature = "git")]
pub fn write_node<N: Fragmentable>(
    repo: &git2::Repository,
    node: &N,
) -> Result<git2::Oid, git2::Error> {
    if node.is_shard() {
        repo.blob(&node.data().encode())
    } else {
        let mut builder = repo.treebuilder(None)?;
        let data_oid = repo.blob(&node.data().encode())?;
        builder.insert(".data", data_oid, 0o100644)?;
        for (i, child) in node.children().iter().enumerate() {
            let child_oid = write_node(repo, child)?;
            let mode = if child.is_shard() { 0o100644 } else { 0o040000 };
            builder.insert(format!("{i:04}"), child_oid, mode)?;
        }
        builder.write()
    }
}

/// Read any Reconstructable from the git object database.
///
/// Git blob → shard. Git tree → node with children (recursive).
/// Requires N: Reconstructable so the type can be rebuilt from parts.
#[cfg(feature = "git")]
pub fn read_node<N: Reconstructable + Clone>(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<N, Box<dyn std::error::Error>>
where
    N::Data: Decode,
{
    use crate::ref_::Ref;

    // Try as blob first (shard).
    if let Ok(blob) = repo.find_blob(oid) {
        let data = N::Data::decode(blob.content()).map_err(|e| format!("decode error: {e}"))?;
        let ref_ = Ref::new(
            <N::Hash as crate::sha::HashAlg>::from_hex(oid.to_string()),
            "shard",
        );
        return Ok(N::reconstruct(ref_, data, vec![]));
    }

    // Otherwise it's a tree (node with children).
    let tree = repo.find_tree(oid)?;
    let data_entry = tree.get_name(".data").ok_or("tree missing .data entry")?;
    let data_blob = repo.find_blob(data_entry.id())?;
    let data = N::Data::decode(data_blob.content()).map_err(|e| format!("decode error: {e}"))?;

    let mut children = Vec::new();
    let mut i = 0;
    while let Some(entry) = tree.get_name(&format!("{i:04}")) {
        let child = read_node::<N>(repo, entry.id())?;
        children.push(child);
        i += 1;
    }

    // Reconstruct and compute the correct content OID.
    let node = N::reconstruct(
        Ref::new(
            <N::Hash as crate::sha::HashAlg>::from_hex(oid.to_string()),
            "node",
        ),
        data,
        children,
    );
    Ok(node)
}

/// Write a commit to git from individual pieces. Returns the commit OID.
#[cfg(feature = "git")]
pub(crate) fn write_commit<E: Encode>(
    repo: &git2::Repository,
    fractal: &Fractal<E>,
    author: &crate::witnessed::Author,
    committer: &crate::witnessed::Committer,
    message: &str,
    parent: Option<&crate::sha::Sha>,
) -> Result<git2::Oid, git2::Error> {
    let tree_oid = match fractal {
        Fractal::Shard { .. } => {
            let blob_oid = write_tree(repo, fractal)?;
            let mut builder = repo.treebuilder(None)?;
            builder.insert(".data", blob_oid, 0o100644)?;
            builder.write()?
        }
        Fractal::Fractal { .. } | Fractal::Lens { .. } => write_tree(repo, fractal)?,
    };
    let tree = repo.find_tree(tree_oid)?;

    let git_author = git2::Signature::now(&author.name, &author.email)?;
    let git_committer = git2::Signature::now(&committer.name, &committer.email)?;

    let parent_commit;
    let parents: Vec<&git2::Commit> = if let Some(parent_sha) = parent {
        let parent_oid = git2::Oid::from_str(&parent_sha.0)?;
        parent_commit = repo.find_commit(parent_oid)?;
        vec![&parent_commit]
    } else {
        vec![]
    };

    repo.commit(None, &git_author, &git_committer, message, &tree, &parents)
}

/// Write a fragment tree using Ref::label as entry names (filesystem mode).
/// Shard -> blob, Fractal -> tree with .data + label-named children.
/// Encoding trees keep the numbered format (write_tree); this is for filesystem trees.
#[cfg(feature = "git")]
pub fn write_tree_named<E: crate::encoding::Encode>(
    repo: &git2::Repository,
    fragment: &Fractal<E>,
) -> Result<git2::Oid, git2::Error> {
    use crate::sha::HashAlg;

    match fragment {
        Fractal::Shard { data, .. } => repo.blob(&data.encode()),
        Fractal::Fractal { data, fractal, .. } => {
            let mut builder = repo.treebuilder(None)?;

            let data_oid = repo.blob(&data.encode())?;
            builder.insert(".data", data_oid, 0o100644)?;

            for child in fractal.iter() {
                let child_oid = write_tree_named(repo, child)?;
                let mode = if child.is_shard() { 0o100644 } else { 0o040000 };
                let name = &child.self_ref().label;
                builder.insert(name.as_str(), child_oid, mode)?;
            }

            builder.write()
        }
        Fractal::Lens { data, target, .. } => {
            let mut builder = repo.treebuilder(None)?;

            let data_oid = repo.blob(&data.encode())?;
            builder.insert(".data", data_oid, 0o100644)?;

            let lens_content: String = target
                .iter()
                .map(|sha| sha.as_str())
                .collect::<Vec<&str>>()
                .join("\n");
            let lens_oid = repo.blob(lens_content.as_bytes())?;
            builder.insert(".lens", lens_oid, 0o100644)?;

            builder.write()
        }
    }
}

/// Reconstruct a Fractal<Vec<u8>> from git objects using entry name as Ref::label.
/// Blob -> Shard with raw bytes, Tree -> Fractal. Children get label from tree entry name.
#[cfg(feature = "git")]
pub fn read_tree_named(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<crate::fragment::Fractal<Vec<u8>>, Box<dyn std::error::Error>> {
    use crate::ref_::Ref;
    use crate::sha::Sha;

    let obj = repo.find_object(oid, None)?;

    match obj.kind() {
        Some(git2::ObjectType::Blob) => {
            let blob = repo.find_blob(oid)?;
            let data = blob.content().to_vec();
            let ref_ = Ref::new(Sha(oid.to_string()), "self");
            Ok(crate::fragment::Fractal::shard_typed(ref_, data))
        }
        Some(git2::ObjectType::Tree) => {
            let tree = repo.find_tree(oid)?;

            let data_entry = tree.get_name(".data").ok_or("tree missing .data entry")?;
            let data_blob = repo.find_blob(data_entry.id())?;
            let data = data_blob.content().to_vec();

            // Check for .lens entry — this is a Lens, not a Fractal
            if let Some(lens_entry) = tree.get_name(".lens") {
                let lens_blob = repo.find_blob(lens_entry.id())?;
                let lens_content = std::str::from_utf8(lens_blob.content())?;
                let targets: Vec<Sha> = lens_content
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| Sha(l.to_string()))
                    .collect();
                let ref_ = Ref::new(Sha(oid.to_string()), "self");
                return Ok(crate::fragment::Fractal::lens_typed(ref_, data, targets));
            }

            let mut children = Vec::new();
            for entry in tree.iter() {
                let name = entry.name().unwrap_or("").to_string();
                if name == ".data" {
                    continue;
                }
                let child = read_tree_named(repo, entry.id())?;
                children.push(relabel_named(child, &name));
            }

            let ref_ = Ref::new(Sha(oid.to_string()), "self");
            Ok(crate::fragment::Fractal::new_typed(ref_, data, children))
        }
        _ => Err(format!("unexpected object type for oid {}", oid).into()),
    }
}

/// Set the label on the top-level Ref of a Fractal<Vec<u8>>.
#[cfg(feature = "git")]
fn relabel_named(
    frag: crate::fragment::Fractal<Vec<u8>>,
    label: &str,
) -> crate::fragment::Fractal<Vec<u8>> {
    use crate::ref_::Ref;
    match frag {
        crate::fragment::Fractal::Shard { ref_, data } => crate::fragment::Fractal::Shard {
            ref_: Ref::new(ref_.sha, label),
            data,
        },
        crate::fragment::Fractal::Fractal {
            ref_,
            data,
            fractal,
        } => crate::fragment::Fractal::Fractal {
            ref_: Ref::new(ref_.sha, label),
            data,
            fractal,
        },
        crate::fragment::Fractal::Lens { ref_, data, target } => crate::fragment::Fractal::Lens {
            ref_: Ref::new(ref_.sha, label),
            data,
            target,
        },
    }
}

/// Reconstruct a Fractal<String> from git objects.
/// Blob -> Shard, Tree -> Fractal. Witness lives on the commit, not the tree.
#[cfg(feature = "git")]
pub fn read_tree(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<Fractal<String>, Box<dyn std::error::Error>> {
    use crate::ref_::Ref;
    use crate::sha::Sha;

    let obj = repo.find_object(oid, None)?;

    match obj.kind() {
        Some(git2::ObjectType::Blob) => {
            let blob = repo.find_blob(oid)?;
            let data = std::str::from_utf8(blob.content())?.to_string();
            let ref_ = Ref::new(Sha(oid.to_string()), "self");
            Ok(Fractal::shard(ref_, data))
        }
        Some(git2::ObjectType::Tree) => {
            let tree = repo.find_tree(oid)?;

            let data_entry = tree.get_name(".data").ok_or("tree missing .data entry")?;
            let data_blob = repo.find_blob(data_entry.id())?;
            let data = std::str::from_utf8(data_blob.content())?.to_string();

            // Check for .lens entry — this is a Lens, not a Fractal
            if let Some(lens_entry) = tree.get_name(".lens") {
                let lens_blob = repo.find_blob(lens_entry.id())?;
                let lens_content = std::str::from_utf8(lens_blob.content())?;
                let targets: Vec<Sha> = lens_content
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| Sha(l.to_string()))
                    .collect();
                let ref_ = Ref::new(Sha(oid.to_string()), "self");
                return Ok(Fractal::lens(ref_, data, targets));
            }

            let mut child_entries: Vec<(String, git2::Oid)> = Vec::new();
            for entry in tree.iter() {
                let name = entry.name().unwrap_or("").to_string();
                if name != ".data" {
                    child_entries.push((name, entry.id()));
                }
            }
            child_entries.sort_by(|a, b| a.0.cmp(&b.0));

            let mut children = Vec::new();
            for (_name, child_oid) in child_entries {
                children.push(read_tree(repo, child_oid)?);
            }

            let ref_ = Ref::new(Sha(oid.to_string()), "self");
            Ok(Fractal::new(ref_, data, children))
        }
        _ => Err(format!("unexpected object type for oid {}", oid).into()),
    }
}

// ---------------------------------------------------------------------------
// GitStore — persistent content-addressed store backed by git
// ---------------------------------------------------------------------------

/// Content-addressed store backed by the git object database.
///
/// Two tiers:
///   Memory: in-process HashMap (hot path, no I/O per operation).
///   Git:    git2 object database (persistent, cloneable, shareable).
///
/// Write: memory only (fast). Call `flush()` to persist to git.
/// Read:  memory first, then git, then miss.
///
/// After `flush()`, everything is in the git repo. `git clone` includes it.
#[cfg(feature = "git")]
pub struct GitStore<N: Fragmentable + Clone> {
    memory: crate::store::Store<N, N::Hash>,
    repo: git2::Repository,
}

#[cfg(feature = "git")]
impl<N: Reconstructable + Clone> GitStore<N>
where
    N::Data: Decode,
{
    /// Open a GitStore backed by the git repo at (or above) the given path.
    pub fn open(path: &std::path::Path) -> Result<Self, git2::Error> {
        let repo = git2::Repository::discover(path)?;
        Ok(GitStore {
            memory: crate::store::Store::new(),
            repo,
        })
    }

    /// Open a GitStore from an existing git2::Repository.
    pub fn from_repo(repo: git2::Repository) -> Self {
        GitStore {
            memory: crate::store::Store::new(),
            repo,
        }
    }

    /// Flush: collapse + write objects + update git ref.
    /// Returns the number of objects written.
    pub fn flush(&self) -> usize {
        use crate::repo::Repo;

        let mut count = 0;

        // Write objects to git ODB.
        for oid in self.memory.keys() {
            if let Some(node) = Repo::read_tree(&self.memory, &oid) {
                if write_node(&self.repo, &node).is_ok() {
                    count += 1;
                }
            }
        }

        // Collapse the index to a single tree, update the ref.
        if let Ok(tree_oid) = self.collapse_index() {
            let _ = self
                .repo
                .reference("refs/store/index", tree_oid, true, "collapse");
        }

        count
    }

    /// Hydrate: read git ref + refract.
    pub fn hydrate(&mut self) {
        let oid = match self
            .repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.target())
        {
            Some(oid) => oid,
            None => return,
        };

        let _ = self.refract_from(oid);
    }

    /// Number of in-memory objects.
    pub fn memory_count(&self) -> usize {
        self.memory.object_count()
    }

    /// Access the underlying git2::Repository.
    pub fn repo(&self) -> &git2::Repository {
        &self.repo
    }

    /// Refract (hydrate) from a specific tree OID.
    fn refract_from(&mut self, oid: git2::Oid) -> Result<(), git2::Error> {
        use crate::repo::Repo;
        use crate::sha::HashAlg;

        let tree = self.repo.find_tree(oid)?;
        for entry in tree.iter() {
            if let Some(name) = entry.name() {
                let target_oid = entry.id().to_string();
                Repo::update_ref(&mut self.memory, name, N::Hash::from_hex(target_oid));
            }
        }
        Ok(())
    }
}

/// Collapse the in-memory ref index to a single git tree OID.
/// Used by higher-level Singularity impls (e.g. EigenStore in coincidence).
#[cfg(feature = "git")]
impl<N: Reconstructable + Clone> GitStore<N>
where
    N::Data: Decode,
{
    /// Collapse: serialize all in-memory refs as entries in one git tree.
    /// Returns the tree OID. Merges with any existing index on disk.
    pub fn collapse_index(&self) -> Result<git2::Oid, git2::Error> {
        use crate::repo::Repo;
        use crate::sha::HashAlg;

        let existing_tree = self
            .repo
            .find_reference("refs/store/index")
            .ok()
            .and_then(|r| r.peel_to_tree().ok());
        let mut builder = self.repo.treebuilder(existing_tree.as_ref())?;

        for ref_name in self.memory.ref_names() {
            if let Some(sha) = Repo::resolve_ref(&self.memory, ref_name) {
                let oid = git2::Oid::from_str(sha.as_str())?;
                builder.insert(ref_name, oid, 0o100644)?;
            }
        }

        builder.write()
    }
}

#[cfg(feature = "git")]
impl<N: Reconstructable + Clone> crate::repo::Repo for GitStore<N>
where
    N::Data: Decode,
{
    type Node = N;
    type Hash = N::Hash;

    fn write_tree(&mut self, node: &N) -> String {
        self.memory.write_tree(node)
    }

    fn read_tree(&self, oid: &str) -> Option<N> {
        // Tier 1: memory.
        if let Some(node) = self.memory.read_tree(oid) {
            return Some(node);
        }
        // Tier 2: git.
        let git_oid = git2::Oid::from_str(oid).ok()?;
        read_node::<N>(&self.repo, git_oid).ok()
    }

    fn write_commit(&mut self, commit: crate::commit::Commit<N, N::Hash>) {
        self.memory.write_commit(commit);
    }

    fn read_commit(&self, sha: &N::Hash) -> Option<crate::commit::Commit<N, N::Hash>> {
        self.memory.read_commit(sha)
    }

    fn update_ref(&mut self, name: &str, sha: N::Hash) {
        self.memory.update_ref(name, sha);
    }

    fn resolve_ref(&self, name: &str) -> Option<N::Hash> {
        // Memory only — call hydrate() on startup to populate from git.
        self.memory.resolve_ref(name)
    }
}
