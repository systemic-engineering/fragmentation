use crate::fragment::{self, Fragment};
use crate::ref_::Ref;
use crate::sha;
use crate::store::Store;
use crate::walk;
use crate::witnessed::Witnessed;

/// Error type for decode failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnknownLabel(String),
}

/// Encode a single character as a Shard.
pub fn encode_char(ch: &str, witness: &Witnessed) -> Fragment {
    let label = format!("utf8/{}", ch);
    let s = sha::Sha(fragment::blob_oid(ch));
    let r = Ref::new(s, label);
    Fragment::shard(r, witness.clone(), ch)
}

/// Encode a word as a Fragment of character Shards.
pub fn encode_word(word: &str, witness: &Witnessed) -> Fragment {
    let chars: Vec<Fragment> = word
        .chars()
        .map(|c| encode_char(&c.to_string(), witness))
        .collect();
    let label = format!("token/{}", word);
    let s = sha::Sha(fragment::tree_oid(word, &chars));
    let r = Ref::new(s, label);
    Fragment::new_fragment(r, witness.clone(), word, chars)
}

/// Encode a sentence as a Fragment of word Fragments.
pub fn encode_sentence(text: &str, witness: &Witnessed) -> Fragment {
    let words: Vec<Fragment> = text
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(|w| encode_word(w, witness))
        .collect();
    let s = sha::Sha(fragment::tree_oid(text, &words));
    let r = Ref::new(s, "sentence");
    Fragment::new_fragment(r, witness.clone(), text, words)
}

/// Encode a paragraph as a Fragment of sentence Fragments.
pub fn encode_paragraph(text: &str, witness: &Witnessed) -> Fragment {
    let sentences: Vec<Fragment> = split_sentences(text)
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| encode_sentence(&s, witness))
        .collect();
    let s = sha::Sha(fragment::tree_oid(text, &sentences));
    let r = Ref::new(s, "paragraph");
    Fragment::new_fragment(r, witness.clone(), text, sentences)
}

/// Encode full text as a document Fragment.
/// Splits on double newlines into paragraphs.
pub fn encode(text: &str, witness: &Witnessed) -> Fragment {
    let paragraphs: Vec<Fragment> = text
        .split("\n\n")
        .filter(|p| !p.is_empty())
        .map(|p| encode_paragraph(p, witness))
        .collect();
    let s = sha::Sha(fragment::tree_oid(text, &paragraphs));
    let r = Ref::new(s, "document");
    Fragment::new_fragment(r, witness.clone(), text, paragraphs)
}

/// Encode and store, returning root Fragment + updated Store (deduped).
pub fn ingest(text: &str, witness: &Witnessed, mut store: Store) -> (Fragment, Store) {
    let root = encode(text, witness);
    for frag in walk::collect(&root) {
        store.put(frag.clone());
    }
    (root, store)
}

/// Decode a Fragment tree back to text.
pub fn decode(fragment: &Fragment) -> Result<String, DecodeError> {
    Ok(fragment.data().to_string())
}

/// Split text into sentences on ". ", "! ", "? " boundaries.
/// Punctuation stays with the preceding sentence.
fn split_sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut current = String::new();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len()
            && (chars[i] == '.' || chars[i] == '!' || chars[i] == '?')
            && chars[i + 1] == ' '
        {
            current.push(chars[i]);
            result.push(current);
            current = String::new();
            i += 2; // skip the space
        } else {
            current.push(chars[i]);
            i += 1;
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}
