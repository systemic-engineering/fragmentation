use fragmentation::encoding::{Decode, Encode};
use fragmentation::fragment::{Fractal, Fragmentable, Reconstructable};
use fragmentation::witnessed::Witnessed;

/// Read witness metadata from any git commit.
/// Returns (Witnessed, Message, tree OID). Works on any commit, not just fragmentation ones.
pub fn read_witnessed(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<(Witnessed, fragmentation::witnessed::Message, git2::Oid), Box<dyn std::error::Error>> {
    use fragmentation::witnessed::{Author, Committer, Message, Timestamp};

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
pub fn read_commit(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<fragmentation::commit::Commit<Fractal<String>>, Box<dyn std::error::Error>> {
    use fragmentation::commit::Parent;
    use fragmentation::sha::Sha;

    let git_commit = repo.find_commit(oid)?;
    let (witnessed, message, tree_oid) = read_witnessed(repo, oid)?;
    let fractal = read_tree(repo, tree_oid)?;
    let sha = Sha(oid.to_string());

    match git_commit.parent_id(0).ok() {
        None => Ok(fragmentation::commit::Commit::full_root(
            fractal, witnessed, message, sha,
        )),
        Some(parent_oid) => Ok(fragmentation::commit::Commit::full_child(
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
pub fn write_tree<E: Encode>(
    repo: &git2::Repository,
    fragment: &Fractal<E>,
) -> Result<git2::Oid, git2::Error> {
    use fragmentation::sha::HashAlg;

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
pub fn read_node<N: Reconstructable + Clone>(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<N, Box<dyn std::error::Error>>
where
    N::Data: Decode,
{
    use fragmentation::ref_::Ref;

    // Try as blob first (shard).
    if let Ok(blob) = repo.find_blob(oid) {
        let data = N::Data::decode(blob.content()).map_err(|e| format!("decode error: {e}"))?;
        let ref_ = Ref::new(
            <N::Hash as fragmentation::sha::HashAlg>::from_hex(oid.to_string()),
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
            <N::Hash as fragmentation::sha::HashAlg>::from_hex(oid.to_string()),
            "node",
        ),
        data,
        children,
    );
    Ok(node)
}

/// Write a commit to git from individual pieces. Returns the commit OID.
pub(crate) fn write_commit<E: Encode>(
    repo: &git2::Repository,
    fractal: &Fractal<E>,
    author: &fragmentation::witnessed::Author,
    committer: &fragmentation::witnessed::Committer,
    message: &str,
    parent: Option<&fragmentation::sha::Sha>,
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
pub fn write_tree_named<E: fragmentation::encoding::Encode>(
    repo: &git2::Repository,
    fragment: &Fractal<E>,
) -> Result<git2::Oid, git2::Error> {
    use fragmentation::sha::HashAlg;

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
pub fn read_tree_named(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<fragmentation::fragment::Fractal<Vec<u8>>, Box<dyn std::error::Error>> {
    use fragmentation::ref_::Ref;
    use fragmentation::sha::Sha;

    let obj = repo.find_object(oid, None)?;

    match obj.kind() {
        Some(git2::ObjectType::Blob) => {
            let blob = repo.find_blob(oid)?;
            let data = blob.content().to_vec();
            let ref_ = Ref::new(Sha(oid.to_string()), "self");
            Ok(fragmentation::fragment::Fractal::shard_typed(ref_, data))
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
                return Ok(fragmentation::fragment::Fractal::lens_typed(
                    ref_, data, targets,
                ));
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
            Ok(fragmentation::fragment::Fractal::new_typed(
                ref_, data, children,
            ))
        }
        _ => Err(format!("unexpected object type for oid {}", oid).into()),
    }
}

/// Set the label on the top-level Ref of a Fractal<Vec<u8>>.
fn relabel_named(
    frag: fragmentation::fragment::Fractal<Vec<u8>>,
    label: &str,
) -> fragmentation::fragment::Fractal<Vec<u8>> {
    use fragmentation::ref_::Ref;
    match frag {
        fragmentation::fragment::Fractal::Shard { ref_, data } => {
            fragmentation::fragment::Fractal::Shard {
                ref_: Ref::new(ref_.sha, label),
                data,
            }
        }
        fragmentation::fragment::Fractal::Fractal {
            ref_,
            data,
            fractal,
        } => fragmentation::fragment::Fractal::Fractal {
            ref_: Ref::new(ref_.sha, label),
            data,
            fractal,
        },
        fragmentation::fragment::Fractal::Lens { ref_, data, target } => {
            fragmentation::fragment::Fractal::Lens {
                ref_: Ref::new(ref_.sha, label),
                data,
                target,
            }
        }
    }
}

/// Reconstruct a Fractal<String> from git objects.
/// Blob -> Shard, Tree -> Fractal. Witness lives on the commit, not the tree.
pub fn read_tree(
    repo: &git2::Repository,
    oid: git2::Oid,
) -> Result<Fractal<String>, Box<dyn std::error::Error>> {
    use fragmentation::ref_::Ref;
    use fragmentation::sha::Sha;

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
                let targets: Vec<fragmentation::sha::Sha> = lens_content
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| fragmentation::sha::Sha(l.to_string()))
                    .collect();
                let ref_ = Ref::new(fragmentation::sha::Sha(oid.to_string()), "self");
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

            let ref_ = Ref::new(fragmentation::sha::Sha(oid.to_string()), "self");
            Ok(Fractal::new(ref_, data, children))
        }
        _ => Err(format!("unexpected object type for oid {}", oid).into()),
    }
}
