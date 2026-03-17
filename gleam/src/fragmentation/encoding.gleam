/// Encoding: text as fragmentation trees.
///
/// Five levels: document → paragraph → sentence → word → character.
/// Every node content-addressed. Lossless round-trip.
/// No witness embedded in the tree — witnesses live at commit level.
import fragmentation.{type Fragment}
import fragmentation/store.{type Store}
import fragmentation/walk
import gleam/list
import gleam/string

/// Error type for decode failures.
pub type DecodeError {
  UnknownLabel(String)
}

/// Hash data with its label namespace to avoid cross-level collisions.
/// A char "a" and word "a" must have different SHAs in the store.
fn labeled_hash(label: String, data: String) -> fragmentation.Sha {
  fragmentation.hash(label <> ":" <> data)
}

/// Encode a single character as a Shard.
pub fn encode_char(char: String) -> Fragment(String) {
  let label = "utf8/" <> char
  let sha = labeled_hash(label, char)
  let r = fragmentation.ref_(sha, label)
  fragmentation.shard(r, char)
}

/// Encode a word as a Fractal of character Shards.
pub fn encode_word(word: String) -> Fragment(String) {
  let chars =
    string.to_graphemes(word)
    |> list.map(fn(c) { encode_char(c) })
  let label = "token/" <> word
  let sha = labeled_hash(label, word)
  let r = fragmentation.ref_(sha, label)
  fragmentation.fractal(r, word, chars)
}

/// Encode a sentence as a Fractal of word Fractals.
pub fn encode_sentence(text: String) -> Fragment(String) {
  let words =
    string.split(text, " ")
    |> list.filter(fn(w) { w != "" })
    |> list.map(fn(w) { encode_word(w) })
  let sha = labeled_hash("sentence", text)
  let r = fragmentation.ref_(sha, "sentence")
  fragmentation.fractal(r, text, words)
}

/// Encode a paragraph as a Fractal of sentence Fractals.
pub fn encode_paragraph(text: String) -> Fragment(String) {
  let sentences =
    split_sentences(text)
    |> list.filter(fn(s) { s != "" })
    |> list.map(fn(s) { encode_sentence(s) })
  let sha = labeled_hash("paragraph", text)
  let r = fragmentation.ref_(sha, "paragraph")
  fragmentation.fractal(r, text, sentences)
}

/// Encode full text as a document Fractal.
/// Splits on double newlines into paragraphs.
pub fn encode(text: String) -> Fragment(String) {
  let paragraphs =
    string.split(text, "\n\n")
    |> list.filter(fn(p) { p != "" })
    |> list.map(fn(p) { encode_paragraph(p) })
  let sha = labeled_hash("document", text)
  let r = fragmentation.ref_(sha, "document")
  fragmentation.fractal(r, text, paragraphs)
}

/// Encode and store, returning root Fragment + updated Store (deduped).
pub fn ingest(
  text: String,
  s: Store(String),
) -> #(Fragment(String), Store(String)) {
  let root = encode(text)
  let updated =
    walk.collect(root)
    |> list.fold(s, fn(acc, frag) {
      store.put(acc, frag, fn(x: String) { x })
    })
  #(root, updated)
}

/// Decode a Fragment tree back to text.
pub fn decode(fragment: Fragment(String)) -> Result(String, DecodeError) {
  Ok(fragmentation.data(fragment))
}

// ---------------------------------------------------------------------------
// Sentence splitting
// ---------------------------------------------------------------------------

/// Split text into sentences on ". ", "! ", "? " boundaries.
/// Punctuation stays with the preceding sentence.
fn split_sentences(text: String) -> List(String) {
  string.to_graphemes(text)
  |> do_split_sentences("", [])
  |> list.reverse
}

fn do_split_sentences(
  chars: List(String),
  current: String,
  acc: List(String),
) -> List(String) {
  case chars {
    [] ->
      case current {
        "" -> acc
        _ -> [current, ..acc]
      }
    [".", " ", ..rest] -> do_split_sentences(rest, "", [current <> ".", ..acc])
    ["!", " ", ..rest] -> do_split_sentences(rest, "", [current <> "!", ..acc])
    ["?", " ", ..rest] -> do_split_sentences(rest, "", [current <> "?", ..acc])
    [c, ..rest] -> do_split_sentences(rest, current <> c, acc)
  }
}
