#[cfg(feature = "git")]
use crate::encoding::Encode;

#[cfg(feature = "git")]
use crate::fragment::{Fractal, Fragment};

#[cfg(feature = "git")]
use crate::witnessed::Witnessed;

/// Read a git commit into its components: Witnessed metadata + tree OID.
/// Works on any git commit, not just fragmentation-written ones.
#[cfg(feature = "git")]
pub fn read_commit(
    _repo: &git2::Repository,
    _oid: git2::Oid,
) -> Result<(Witnessed, git2::Oid), Box<dyn std::error::Error>> {
    todo!()
}

/// Extract the signature from a signed commit, if present.
/// Returns None for unsigned commits.
#[cfg(feature = "git")]
pub fn commit_signature(
    _repo: &git2::Repository,
    _oid: git2::Oid,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    todo!()
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

/// Write a fragment and commit it. Returns the commit OID.
/// Witnessed fields map to git author/committer. Message is pass-through.
#[cfg(feature = "git")]
pub fn write_commit<E: Encode>(
    repo: &git2::Repository,
    fragment: &Fractal<E>,
    witnessed: &Witnessed,
    message: &str,
    parent: Option<&git2::Commit>,
) -> Result<git2::Oid, git2::Error> {
    let tree_oid = match fragment {
        Fractal::Shard { .. } => {
            let blob_oid = write_tree(repo, fragment)?;
            let mut builder = repo.treebuilder(None)?;
            builder.insert(".data", blob_oid, 0o100644)?;
            builder.write()?
        }
        Fractal::Fractal { .. } => write_tree(repo, fragment)?,
    };
    let tree = repo.find_tree(tree_oid)?;

    let author = git2::Signature::now(
        &witnessed.author.0,
        &format!("{}@systemic.engineer", witnessed.author.0),
    )?;
    let committer = git2::Signature::now(
        &witnessed.committer.0,
        &format!("{}@systemic.engineer", witnessed.committer.0),
    )?;

    let parents: Vec<&git2::Commit> = parent.into_iter().collect();
    repo.commit(None, &author, &committer, message, &tree, &parents)
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
