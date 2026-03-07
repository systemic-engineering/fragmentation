use fragmentation::diff;
use fragmentation::encoding;
use fragmentation::fragment;
use fragmentation::store::Store;

// ===========================================================================
// Round 1: Character -> Shard
// ===========================================================================

#[test]
fn encode_char_is_shard() {
    let result = encoding::encode_char("a");
    assert!(result.is_shard());
}

#[test]
fn encode_char_data() {
    let result = encoding::encode_char("a");
    assert_eq!(result.data(), "a");
}

#[test]
fn encode_char_label() {
    let result = encoding::encode_char("a");
    assert_eq!(result.self_ref().label, "utf8/a");
}

#[test]
fn encode_char_deterministic() {
    let a = encoding::encode_char("a");
    let b = encoding::encode_char("a");
    assert_eq!(fragment::content_oid(&a), fragment::content_oid(&b));
}

#[test]
fn encode_char_multibyte() {
    let result = encoding::encode_char("é");
    assert_eq!(result.data(), "é");
    assert_eq!(result.self_ref().label, "utf8/é");
}

#[test]
fn encode_char_different_chars_different_oid() {
    let a = encoding::encode_char("a");
    let b = encoding::encode_char("b");
    assert_ne!(fragment::content_oid(&a), fragment::content_oid(&b));
}

#[test]
fn encode_char_oid_is_blob_oid() {
    let a = encoding::encode_char("a");
    assert_eq!(a.self_ref().sha.0, fragment::blob_oid("a"));
}

// ===========================================================================
// Round 2: Word -> Fragment of char Shards
// ===========================================================================

#[test]
fn encode_word_is_fractal() {
    let result = encoding::encode_word("hi");
    assert!(result.is_fractal());
}

#[test]
fn encode_word_data() {
    let result = encoding::encode_word("hi");
    assert_eq!(result.data(), "hi");
}

#[test]
fn encode_word_label() {
    let result = encoding::encode_word("hi");
    assert_eq!(result.self_ref().label, "token/hi");
}

#[test]
fn encode_word_children_are_char_shards() {
    let result = encoding::encode_word("hi");
    let children = result.children();
    assert_eq!(children.len(), 2);
    assert!(children.iter().all(|c| c.is_shard()));
}

#[test]
fn encode_word_children_data() {
    let result = encoding::encode_word("hi");
    let children = result.children();
    let data: Vec<&str> = children.iter().map(|c| c.data()).collect();
    assert_eq!(data, vec!["h", "i"]);
}

#[test]
fn decode_word_roundtrip() {
    let word = encoding::encode_word("hi");
    assert_eq!(encoding::decode(&word), Ok("hi".to_string()));
}

#[test]
fn encode_word_oid_matches_content_oid() {
    let word = encoding::encode_word("hi");
    assert_eq!(word.self_ref().sha.0, fragment::content_oid(&word));
}

// ===========================================================================
// Round 3: Paragraph -> Fragment of sentence Fragments
// ===========================================================================

#[test]
fn encode_paragraph_is_fractal() {
    let result = encoding::encode_paragraph("hi reed");
    assert!(result.is_fractal());
}

#[test]
fn encode_paragraph_label() {
    let result = encoding::encode_paragraph("hi reed");
    assert_eq!(result.self_ref().label, "paragraph");
}

#[test]
fn encode_paragraph_children_are_sentences() {
    let result = encoding::encode_paragraph("hi reed");
    let children = result.children();
    assert_eq!(children.len(), 1);
    assert!(children.iter().all(|c| c.is_fractal()));
}

#[test]
fn encode_paragraph_sentence_labels() {
    let result = encoding::encode_paragraph("hi reed");
    let labels: Vec<&str> = result
        .children()
        .iter()
        .map(|f| f.self_ref().label.as_str())
        .collect();
    assert_eq!(labels, vec!["sentence"]);
}

#[test]
fn decode_paragraph_roundtrip() {
    let para = encoding::encode_paragraph("hi reed");
    assert_eq!(encoding::decode(&para), Ok("hi reed".to_string()));
}

#[test]
fn encode_paragraph_filters_empty_words() {
    let result = encoding::encode_paragraph("hi  reed");
    let sentences = result.children();
    assert_eq!(sentences.len(), 1);
    let words = sentences[0].children();
    assert_eq!(words.len(), 2);
}

// ===========================================================================
// Round 3b: Sentence -> Fragment of word Fragments
// ===========================================================================

#[test]
fn encode_sentence_is_fractal() {
    let result = encoding::encode_sentence("hello world");
    assert!(result.is_fractal());
}

#[test]
fn encode_sentence_label() {
    let result = encoding::encode_sentence("hello world");
    assert_eq!(result.self_ref().label, "sentence");
}

#[test]
fn encode_sentence_data() {
    let result = encoding::encode_sentence("hello world");
    assert_eq!(result.data(), "hello world");
}

#[test]
fn encode_sentence_children_are_words() {
    let result = encoding::encode_sentence("hello world");
    let children = result.children();
    assert_eq!(children.len(), 2);
    let labels: Vec<&str> = children
        .iter()
        .map(|f| f.self_ref().label.as_str())
        .collect();
    assert_eq!(labels, vec!["token/hello", "token/world"]);
}

#[test]
fn decode_sentence_roundtrip() {
    let s = encoding::encode_sentence("hello world");
    assert_eq!(encoding::decode(&s), Ok("hello world".to_string()));
}

#[test]
fn encode_paragraph_splits_sentences() {
    let result = encoding::encode_paragraph("Hello world. How are you?");
    let sentences = result.children();
    assert_eq!(sentences.len(), 2);
    assert_eq!(sentences[0].data(), "Hello world.");
}

#[test]
fn encode_paragraph_single_sentence() {
    let result = encoding::encode_paragraph("no punctuation here");
    assert_eq!(result.children().len(), 1);
}

#[test]
fn encode_paragraph_exclamation_split() {
    let result = encoding::encode_paragraph("Wow! That works.");
    let sentences = result.children();
    assert_eq!(sentences.len(), 2);
    assert_eq!(sentences[0].data(), "Wow!");
}

#[test]
fn encode_paragraph_question_split() {
    let result = encoding::encode_paragraph("Really? Yes.");
    assert_eq!(result.children().len(), 2);
}

#[test]
fn encode_paragraph_sentence_has_words() {
    let result = encoding::encode_paragraph("Hi Reed. Bye Reed.");
    let sentences = result.children();
    let first = &sentences[0];
    let words = first.children();
    assert_eq!(words.len(), 2);
    let labels: Vec<&str> = words.iter().map(|f| f.self_ref().label.as_str()).collect();
    assert_eq!(labels, vec!["token/Hi", "token/Reed."]);
}

// ===========================================================================
// Round 4: Full text round-trip
// ===========================================================================

#[test]
fn encode_is_fractal() {
    let result = encoding::encode("Hi Reed.\n\nHow are you?");
    assert!(result.is_fractal());
}

#[test]
fn encode_label_is_document() {
    let result = encoding::encode("Hi Reed.\n\nHow are you?");
    assert_eq!(result.self_ref().label, "document");
}

#[test]
fn encode_two_paragraphs() {
    let result = encoding::encode("Hi Reed.\n\nHow are you?");
    assert_eq!(result.children().len(), 2);
}

#[test]
fn encode_paragraph_labels() {
    let result = encoding::encode("Hi Reed.\n\nHow are you?");
    assert!(result
        .children()
        .iter()
        .all(|f| f.self_ref().label == "paragraph"));
}

#[test]
fn decode_full_roundtrip() {
    let text = "Hi Reed.\n\nHow are you?";
    let doc = encoding::encode(text);
    assert_eq!(encoding::decode(&doc), Ok(text.to_string()));
}

#[test]
fn encode_single_paragraph() {
    let result = encoding::encode("just one");
    assert_eq!(result.children().len(), 1);
}

#[test]
fn encode_empty_text() {
    let result = encoding::encode("");
    assert!(result.children().is_empty());
    assert_eq!(result.data(), "");
}

#[test]
fn encode_document_oid_matches_content_oid() {
    let doc = encoding::encode("hello world");
    assert_eq!(doc.self_ref().sha.0, fragment::content_oid(&doc));
}

// ===========================================================================
// Round 5: Deduplication via ingest + Store
// ===========================================================================

#[test]
fn ingest_returns_root_and_store() {
    let s = Store::new();
    let (root, updated) = encoding::ingest("hello", s);
    assert!(root.is_fractal());
    assert!(updated.size() > 0);
}

#[test]
fn ingest_deduplicates_repeated_words() {
    let s = Store::new();
    let (_root, updated) = encoding::ingest("the the the", s);
    assert_eq!(updated.size(), 7);
}

#[test]
fn ingest_all_unique_words() {
    let s = Store::new();
    let (_root, updated) = encoding::ingest("a b", s);
    assert_eq!(updated.size(), 7);
}

#[test]
fn ingest_preserves_existing_store() {
    let s = Store::new();
    let (_r1, s1) = encoding::ingest("hi", s);
    let (_r2, s2) = encoding::ingest("hi there", s1);
    assert_eq!(s2.size(), 13);
}

// ===========================================================================
// Round 6: Diff at word granularity
// ===========================================================================

#[test]
fn diff_same_document_unchanged() {
    let doc = encoding::encode("hello world");
    let changes = diff::diff(&doc, &doc);
    assert_eq!(changes, vec![diff::Change::Unchanged(doc)]);
}

#[test]
fn diff_modified_word() {
    let doc_a = encoding::encode("hello world");
    let doc_b = encoding::encode("hello reed");
    let changes = diff::diff(&doc_a, &doc_b);
    let (_, _, modified, unchanged) = diff::summary(&changes);
    assert!(unchanged > 0);
    assert!(modified > 0);
}

#[test]
fn diff_first_word_unchanged() {
    let doc_a = encoding::encode("hello world");
    let doc_b = encoding::encode("hello reed");
    let changes = diff::diff(&doc_a, &doc_b);
    let has_hello_unchanged = changes.iter().any(|c| match c {
        diff::Change::Unchanged(f) => f.data() == "hello",
        _ => false,
    });
    assert!(has_hello_unchanged);
}

#[test]
fn diff_added_paragraph() {
    let doc_a = encoding::encode("hello");
    let doc_b = encoding::encode("hello\n\nworld");
    let changes = diff::diff(&doc_a, &doc_b);
    assert!(changes.iter().any(|c| matches!(c, diff::Change::Added(_))));
}
