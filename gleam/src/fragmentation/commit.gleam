/// Commit: witnesses, drafts, and commits.
///
/// Witnessed lives at the commit level only.
/// Different witness → different commit SHA.
/// Same content, different observer → different commit, same tree OID.
import fragmentation
import gleam/option.{type Option, None, Some}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Who wrote the content. Who made the decision. Who holds the intent.
pub type Author {
  Author(name: String, email: String)
}

/// Who ran the process. Who executed. Who was the mechanism.
pub type Committer {
  Committer(name: String, email: String)
}

/// When the observation happened. Opaque string: ISO 8601, epoch, logical clock.
pub type Timestamp {
  Timestamp(value: String)
}

/// The witness's account of what happened.
pub type Message {
  Message(value: String)
}

/// Git commit metadata. Who was here when this happened.
pub type Witnessed {
  Witnessed(author: Author, committer: Committer, timestamp: Timestamp)
}

/// A reference to a parent commit.
pub type Parent {
  Parent(sha: fragmentation.Sha)
}

/// Uncommitted intent: node + message + optional parent + optional author.
pub type Draft(node) {
  Draft(
    node: node,
    message: Message,
    parent: Option(Parent),
    author: Option(Author),
  )
}

/// A finalized commit. Root has no parent; Child has one.
pub type Commit(node) {
  Root(
    node: node,
    witnessed: Witnessed,
    message: Message,
    sha: fragmentation.Sha,
  )
  Child(
    node: node,
    witnessed: Witnessed,
    message: Message,
    parent: Parent,
    sha: fragmentation.Sha,
  )
}

// ---------------------------------------------------------------------------
// Draft construction
// ---------------------------------------------------------------------------

/// Create a root draft (no parent).
pub fn root(message: String, node: node) -> Draft(node) {
  Draft(node: node, message: Message(message), parent: None, author: None)
}

/// Create a child draft (with parent).
pub fn child(message: String, node: node, parent: Parent) -> Draft(node) {
  Draft(
    node: node,
    message: Message(message),
    parent: Some(parent),
    author: None,
  )
}

/// Set the author on a draft.
pub fn authored(draft: Draft(node), author: Author) -> Draft(node) {
  Draft(..draft, author: Some(author))
}

// ---------------------------------------------------------------------------
// Finalization
// ---------------------------------------------------------------------------

/// Finalize a draft into a Commit. Computes witness and SHA-512 commit SHA.
/// If no author is set, committer acts as author (git convention).
pub fn commit(
  draft: Draft(node),
  committer: Committer,
  timestamp: Timestamp,
  hash_node: fn(node) -> String,
) -> Commit(node) {
  let author = case draft.author {
    Some(a) -> a
    None -> Author(committer.name, committer.email)
  }
  let w = Witnessed(author: author, committer: committer, timestamp: timestamp)
  let content_oid = hash_node(draft.node)
  let sha_str = compute_sha(w, draft.message, draft.parent, content_oid)
  let sha = fragmentation.sha(sha_str)
  case draft.parent {
    None ->
      Root(node: draft.node, witnessed: w, message: draft.message, sha: sha)
    Some(parent) ->
      Child(
        node: draft.node,
        witnessed: w,
        message: draft.message,
        parent: parent,
        sha: sha,
      )
  }
}

fn compute_sha(
  w: Witnessed,
  msg: Message,
  parent: Option(Parent),
  content_oid: String,
) -> String {
  let Author(aname, aemail) = w.author
  let Committer(cname, cemail) = w.committer
  let Timestamp(ts) = w.timestamp
  let Message(m) = msg
  let parent_str = case parent {
    None -> ""
    Some(Parent(fragmentation.Sha(s))) -> "\nparent:" <> s
  }
  let text =
    "author:"
    <> aname
    <> "<"
    <> aemail
    <> ">"
    <> "\ncommitter:"
    <> cname
    <> "<"
    <> cemail
    <> ">"
    <> "\ntimestamp:"
    <> ts
    <> "\nmessage:"
    <> m
    <> parent_str
    <> "\ntree:"
    <> content_oid
  let fragmentation.Sha(sha_str) = fragmentation.hash(text)
  sha_str
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

/// Extract the SHA from a commit.
pub fn sha(c: Commit(node)) -> fragmentation.Sha {
  case c {
    Root(sha: s, ..) -> s
    Child(sha: s, ..) -> s
  }
}

/// Extract the witness from a commit.
pub fn witnessed(c: Commit(node)) -> Witnessed {
  case c {
    Root(witnessed: w, ..) -> w
    Child(witnessed: w, ..) -> w
  }
}

/// Extract the node from a commit.
pub fn node(c: Commit(node)) -> node {
  case c {
    Root(node: n, ..) -> n
    Child(node: n, ..) -> n
  }
}

/// Extract the message from a commit.
pub fn message(c: Commit(node)) -> Message {
  case c {
    Root(message: m, ..) -> m
    Child(message: m, ..) -> m
  }
}

/// Extract the parent. None for Root, Some for Child.
pub fn parent(c: Commit(node)) -> Option(Parent) {
  case c {
    Root(..) -> None
    Child(parent: p, ..) -> Some(p)
  }
}
