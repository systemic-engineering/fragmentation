use std::convert::Infallible;

use crate::fragment::{self, Fragment};
use crate::ref_::Ref;
use crate::sha;
use crate::store::Store;
use crate::walk;

// ===========================================================================
// Encode / Decode traits
// ===========================================================================

pub trait Encode {
    fn encode(&self) -> Vec<u8>;
}

pub trait Decode: Sized {
    type Error: std::fmt::Display;
    fn decode(bytes: &[u8]) -> Result<Self, Self::Error>;
}

impl Encode for Vec<u8> {
    fn encode(&self) -> Vec<u8> {
        self.clone()
    }
}

impl Decode for Vec<u8> {
    type Error = Infallible;
    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(bytes.to_vec())
    }
}

impl Encode for String {
    fn encode(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl Decode for String {
    type Error = std::string::FromUtf8Error;
    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        String::from_utf8(bytes.to_vec())
    }
}

// ===========================================================================
// Text encoding (five-level trees: document/paragraph/sentence/word/char)
// ===========================================================================

/// Error type for decode failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnknownLabel(String),
}

/// Encode a single character as a Shard.
pub fn encode_char(ch: &str) -> Fragment {
    let label = format!("utf8/{}", ch);
    let s = sha::Sha(fragment::blob_oid(ch));
    let r = Ref::new(s, label);
    Fragment::shard(r, ch.to_string())
}

/// Encode a word as a Fragment of character Shards.
pub fn encode_word(word: &str) -> Fragment {
    let chars: Vec<Fragment> = word.chars().map(|c| encode_char(&c.to_string())).collect();
    let label = format!("token/{}", word);
    let s = sha::Sha(fragment::tree_oid(word, &chars));
    let r = Ref::new(s, label);
    Fragment::fractal(r, word.to_string(), chars)
}

/// Encode a sentence as a Fragment of word Fragments.
pub fn encode_sentence(text: &str) -> Fragment {
    let words: Vec<Fragment> = text
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(encode_word)
        .collect();
    let s = sha::Sha(fragment::tree_oid(text, &words));
    let r = Ref::new(s, "sentence");
    Fragment::fractal(r, text.to_string(), words)
}

/// Encode a paragraph as a Fragment of sentence Fragments.
pub fn encode_paragraph(text: &str) -> Fragment {
    let sentences: Vec<Fragment> = split_sentences(text)
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| encode_sentence(&s))
        .collect();
    let s = sha::Sha(fragment::tree_oid(text, &sentences));
    let r = Ref::new(s, "paragraph");
    Fragment::fractal(r, text.to_string(), sentences)
}

/// Encode full text as a document Fragment.
/// Splits on double newlines into paragraphs.
pub fn encode(text: &str) -> Fragment {
    let paragraphs: Vec<Fragment> = text
        .split("\n\n")
        .filter(|p| !p.is_empty())
        .map(encode_paragraph)
        .collect();
    let s = sha::Sha(fragment::tree_oid(text, &paragraphs));
    let r = Ref::new(s, "document");
    Fragment::fractal(r, text.to_string(), paragraphs)
}

/// Encode and store, returning root Fragment + updated Store (deduped).
pub fn ingest(text: &str, mut store: Store) -> (Fragment, Store) {
    let root = encode(text);
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
            i += 2;
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
