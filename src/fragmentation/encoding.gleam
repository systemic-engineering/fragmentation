/// Encoding: text as fragmentation trees.
///
/// Five levels: document → paragraph → sentence → word → character.
/// Every node content-addressed. Lossless round-trip.
import fragmentation.{type Fragment, type Witnessed}
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
pub fn encode_char(char: String, witness: Witnessed) -> Fragment {
  let label = "utf8/" <> char
  let sha = labeled_hash(label, char)
  let r = fragmentation.ref(sha, label)
  fragmentation.shard(r, witness, char)
}

/// Encode a word as a Fragment of character Shards.
pub fn encode_word(word: String, witness: Witnessed) -> Fragment {
  let chars =
    string.to_graphemes(word)
    |> list.map(fn(c) { encode_char(c, witness) })
  let label = "token/" <> word
  let sha = labeled_hash(label, word)
  let r = fragmentation.ref(sha, label)
  fragmentation.fragment(r, witness, word, chars)
}

/// Encode a sentence as a Fragment of word Fragments.
pub fn encode_sentence(text: String, witness: Witnessed) -> Fragment {
  let words =
    string.split(text, " ")
    |> list.filter(fn(w) { w != "" })
    |> list.map(fn(w) { encode_word(w, witness) })
  let sha = labeled_hash("sentence", text)
  let r = fragmentation.ref(sha, "sentence")
  fragmentation.fragment(r, witness, text, words)
}

/// Encode a paragraph as a Fragment of sentence Fragments.
pub fn encode_paragraph(text: String, witness: Witnessed) -> Fragment {
  let sentences =
    split_sentences(text)
    |> list.filter(fn(s) { s != "" })
    |> list.map(fn(s) { encode_sentence(s, witness) })
  let sha = labeled_hash("paragraph", text)
  let r = fragmentation.ref(sha, "paragraph")
  fragmentation.fragment(r, witness, text, sentences)
}

/// Encode full text as a document Fragment.
/// Splits on double newlines into paragraphs.
pub fn encode(text: String, witness: Witnessed) -> Fragment {
  let paragraphs =
    string.split(text, "\n\n")
    |> list.filter(fn(p) { p != "" })
    |> list.map(fn(p) { encode_paragraph(p, witness) })
  let sha = labeled_hash("document", text)
  let r = fragmentation.ref(sha, "document")
  fragmentation.fragment(r, witness, text, paragraphs)
}

/// Encode and store, returning root Fragment + updated Store (deduped).
pub fn ingest(text: String, witness: Witnessed, s: Store) -> #(Fragment, Store) {
  let root = encode(text, witness)
  let updated =
    walk.collect(root)
    |> list.fold(s, fn(acc, frag) { store.put(acc, frag) })
  #(root, updated)
}

/// Decode a Fragment tree back to text.
pub fn decode(fragment: Fragment) -> Result(String, DecodeError) {
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
