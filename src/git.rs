#[cfg(feature = "git")]
use crate::encoding::Encode;

#[cfg(feature = "git")]
use crate::fragment::{Fractal, Fragment};

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

/// Read a fragmentation commit. Returns Commit<String> with full metadata and fractal tree.
/// Only works on commits written by write_commit (fragmentation-format trees).
#[cfg(feature = "git")]
pub fn read_commit(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<crate::commit::Commit<String>, Box<dyn std::error::Error>> {
    use crate::sha::Sha;

    let git_commit = repo.find_commit(oid)?;
    let (witnessed, message, tree_oid) = read_witnessed(repo, oid)?;
    let fractal = read_tree(repo, tree_oid)?;
    let parent = git_commit.parent_id(0).ok().map(|id| Sha(id.to_string()));
    Ok(crate::commit::Commit::full(
        fractal,
        witnessed,
        message,
        parent,
        Sha(oid.to_string()),
    ))
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
    }
}

/// Write a Commit to git. Returns the commit OID.
/// Witnessed fields map to git author/committer. Message is pass-through.
#[cfg(feature = "git")]
pub fn write_commit<E: Encode>(
    repo: &git2::Repository,
    commit: &crate::commit::Commit<E>,
) -> Result<git2::Oid, git2::Error> {
    let tree_oid = match commit.fractal() {
        Fractal::Shard { .. } => {
            let blob_oid = write_tree(repo, commit.fractal())?;
            let mut builder = repo.treebuilder(None)?;
            builder.insert(".data", blob_oid, 0o100644)?;
            builder.write()?
        }
        Fractal::Fractal { .. } => write_tree(repo, commit.fractal())?,
    };
    let tree = repo.find_tree(tree_oid)?;

    let author = git2::Signature::now(
        &commit.witnessed().author.name,
        &commit.witnessed().author.email,
    )?;
    let committer = git2::Signature::now(
        &commit.witnessed().committer.name,
        &commit.witnessed().committer.email,
    )?;

    let parent_commit;
    let parents: Vec<&git2::Commit> = if let Some(parent_sha) = commit.parent() {
        let parent_oid = git2::Oid::from_str(&parent_sha.0)?;
        parent_commit = repo.find_commit(parent_oid)?;
        vec![&parent_commit]
    } else {
        vec![]
    };

    repo.commit(
        None,
        &author,
        &committer,
        &commit.message().0,
        &tree,
        &parents,
    )
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
