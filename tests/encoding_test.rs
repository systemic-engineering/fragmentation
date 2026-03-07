use fragmentation::diff;
use fragmentation::encoding;
use fragmentation::fragment;
use fragmentation::store::Store;
use fragmentation::witnessed::{Author, Committer, Message, Timestamp, Witnessed};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_witnessed() -> Witnessed {
    Witnessed::new(
        Author("alex".into()),
        Committer("reed".into()),
        Timestamp("2026-03-01T00:00:00Z".into()),
        Message("test".into()),
    )
}

// ===========================================================================
// Round 1: Character -> Shard
// ===========================================================================

#[test]
fn encode_char_is_shard() {
    let w = test_witnessed();
    let result = encoding::encode_char("a", &w);
    assert!(result.is_shard());
}

#[test]
fn encode_char_data() {
    let w = test_witnessed();
    let result = encoding::encode_char("a", &w);
    assert_eq!(result.data(), "a");
}

#[test]
fn encode_char_label() {
    let w = test_witnessed();
    let result = encoding::encode_char("a", &w);
    assert_eq!(result.self_ref().label, "utf8/a");
}

#[test]
fn encode_char_deterministic() {
    let w = test_witnessed();
    let a = encoding::encode_char("a", &w);
    let b = encoding::encode_char("a", &w);
    assert_eq!(fragment::content_oid(&a), fragment::content_oid(&b));
}

#[test]
fn encode_char_multibyte() {
    let w = test_witnessed();
    let result = encoding::encode_char("é", &w);
    assert_eq!(result.data(), "é");
    assert_eq!(result.self_ref().label, "utf8/é");
}

#[test]
fn encode_char_different_chars_different_oid() {
    let w = test_witnessed();
    let a = encoding::encode_char("a", &w);
    let b = encoding::encode_char("b", &w);
    assert_ne!(fragment::content_oid(&a), fragment::content_oid(&b));
}

#[test]
fn encode_char_oid_is_blob_oid() {
    let w = test_witnessed();
    let a = encoding::encode_char("a", &w);
    assert_eq!(a.self_ref().sha.0, fragment::blob_oid("a"));
}

// ===========================================================================
// Round 2: Word -> Fragment of char Shards
// ===========================================================================

#[test]
fn encode_word_is_fragment() {
    let w = test_witnessed();
    let result = encoding::encode_word("hi", &w);
    assert!(result.is_fragment());
}

#[test]
fn encode_word_data() {
    let w = test_witnessed();
    let result = encoding::encode_word("hi", &w);
    assert_eq!(result.data(), "hi");
}

#[test]
fn encode_word_label() {
    let w = test_witnessed();
    let result = encoding::encode_word("hi", &w);
    assert_eq!(result.self_ref().label, "token/hi");
}

#[test]
fn encode_word_children_are_char_shards() {
    let w = test_witnessed();
    let result = encoding::encode_word("hi", &w);
    let children = result.children();
    assert_eq!(children.len(), 2);
    assert!(children.iter().all(|c| c.is_shard()));
}

#[test]
fn encode_word_children_data() {
    let w = test_witnessed();
    let result = encoding::encode_word("hi", &w);
    let children = result.children();
    let data: Vec<&str> = children.iter().map(|c| c.data()).collect();
    assert_eq!(data, vec!["h", "i"]);
}

#[test]
fn decode_word_roundtrip() {
    let w = test_witnessed();
    let word = encoding::encode_word("hi", &w);
    assert_eq!(encoding::decode(&word), Ok("hi".to_string()));
}

#[test]
fn encode_word_oid_matches_content_oid() {
    let w = test_witnessed();
    let word = encoding::encode_word("hi", &w);
    assert_eq!(word.self_ref().sha.0, fragment::content_oid(&word));
}

// ===========================================================================
// Round 3: Paragraph -> Fragment of sentence Fragments
// ===========================================================================

#[test]
fn encode_paragraph_is_fragment() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("hi reed", &w);
    assert!(result.is_fragment());
}

#[test]
fn encode_paragraph_label() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("hi reed", &w);
    assert_eq!(result.self_ref().label, "paragraph");
}

#[test]
fn encode_paragraph_children_are_sentences() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("hi reed", &w);
    let children = result.children();
    assert_eq!(children.len(), 1);
    assert!(children.iter().all(|c| c.is_fragment()));
}

#[test]
fn encode_paragraph_sentence_labels() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("hi reed", &w);
    let labels: Vec<&str> = result
        .children()
        .iter()
        .map(|f| f.self_ref().label.as_str())
        .collect();
    assert_eq!(labels, vec!["sentence"]);
}

#[test]
fn decode_paragraph_roundtrip() {
    let w = test_witnessed();
    let para = encoding::encode_paragraph("hi reed", &w);
    assert_eq!(encoding::decode(&para), Ok("hi reed".to_string()));
}

#[test]
fn encode_paragraph_filters_empty_words() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("hi  reed", &w);
    let sentences = result.children();
    assert_eq!(sentences.len(), 1);
    let words = sentences[0].children();
    assert_eq!(words.len(), 2);
}

// ===========================================================================
// Round 3b: Sentence -> Fragment of word Fragments
// ===========================================================================

#[test]
fn encode_sentence_is_fragment() {
    let w = test_witnessed();
    let result = encoding::encode_sentence("hello world", &w);
    assert!(result.is_fragment());
}

#[test]
fn encode_sentence_label() {
    let w = test_witnessed();
    let result = encoding::encode_sentence("hello world", &w);
    assert_eq!(result.self_ref().label, "sentence");
}

#[test]
fn encode_sentence_data() {
    let w = test_witnessed();
    let result = encoding::encode_sentence("hello world", &w);
    assert_eq!(result.data(), "hello world");
}

#[test]
fn encode_sentence_children_are_words() {
    let w = test_witnessed();
    let result = encoding::encode_sentence("hello world", &w);
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
    let w = test_witnessed();
    let s = encoding::encode_sentence("hello world", &w);
    assert_eq!(encoding::decode(&s), Ok("hello world".to_string()));
}

#[test]
fn encode_paragraph_splits_sentences() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("Hello world. How are you?", &w);
    let sentences = result.children();
    assert_eq!(sentences.len(), 2);
    assert_eq!(sentences[0].data(), "Hello world.");
}

#[test]
fn encode_paragraph_single_sentence() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("no punctuation here", &w);
    assert_eq!(result.children().len(), 1);
}

#[test]
fn encode_paragraph_exclamation_split() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("Wow! That works.", &w);
    let sentences = result.children();
    assert_eq!(sentences.len(), 2);
    assert_eq!(sentences[0].data(), "Wow!");
}

#[test]
fn encode_paragraph_question_split() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("Really? Yes.", &w);
    assert_eq!(result.children().len(), 2);
}

#[test]
fn encode_paragraph_sentence_has_words() {
    let w = test_witnessed();
    let result = encoding::encode_paragraph("Hi Reed. Bye Reed.", &w);
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
fn encode_is_fragment() {
    let w = test_witnessed();
    let result = encoding::encode("Hi Reed.\n\nHow are you?", &w);
    assert!(result.is_fragment());
}

#[test]
fn encode_label_is_document() {
    let w = test_witnessed();
    let result = encoding::encode("Hi Reed.\n\nHow are you?", &w);
    assert_eq!(result.self_ref().label, "document");
}

#[test]
fn encode_two_paragraphs() {
    let w = test_witnessed();
    let result = encoding::encode("Hi Reed.\n\nHow are you?", &w);
    assert_eq!(result.children().len(), 2);
}

#[test]
fn encode_paragraph_labels() {
    let w = test_witnessed();
    let result = encoding::encode("Hi Reed.\n\nHow are you?", &w);
    assert!(result
        .children()
        .iter()
        .all(|f| f.self_ref().label == "paragraph"));
}

#[test]
fn decode_full_roundtrip() {
    let w = test_witnessed();
    let text = "Hi Reed.\n\nHow are you?";
    let doc = encoding::encode(text, &w);
    assert_eq!(encoding::decode(&doc), Ok(text.to_string()));
}

#[test]
fn encode_single_paragraph() {
    let w = test_witnessed();
    let result = encoding::encode("just one", &w);
    assert_eq!(result.children().len(), 1);
}

#[test]
fn encode_empty_text() {
    let w = test_witnessed();
    let result = encoding::encode("", &w);
    assert!(result.children().is_empty());
    assert_eq!(result.data(), "");
}

#[test]
fn encode_document_oid_matches_content_oid() {
    let w = test_witnessed();
    let doc = encoding::encode("hello world", &w);
    assert_eq!(doc.self_ref().sha.0, fragment::content_oid(&doc));
}

// ===========================================================================
// Round 5: Deduplication via ingest + Store
// ===========================================================================

#[test]
fn ingest_returns_root_and_store() {
    let w = test_witnessed();
    let s = Store::new();
    let (root, updated) = encoding::ingest("hello", &w, s);
    assert!(root.is_fragment());
    assert!(updated.size() > 0);
}

#[test]
fn ingest_deduplicates_repeated_words() {
    let w = test_witnessed();
    let s = Store::new();
    let (_root, updated) = encoding::ingest("the the the", &w, s);
    // 3 unique chars (t, h, e) + 1 word ("the") + 1 sentence + 1 paragraph + 1 document = 7
    assert_eq!(updated.size(), 7);
}

#[test]
fn ingest_all_unique_words() {
    let w = test_witnessed();
    let s = Store::new();
    let (_root, updated) = encoding::ingest("a b", &w, s);
    // chars: a, b = 2, words: "a", "b" = 2, sentence = 1, paragraph = 1, document = 1 = 7
    assert_eq!(updated.size(), 7);
}

#[test]
fn ingest_preserves_existing_store() {
    let w = test_witnessed();
    let s = Store::new();
    let (_r1, s1) = encoding::ingest("hi", &w, s);
    let (_r2, s2) = encoding::ingest("hi there", &w, s1);
    // First: h, i = 2 chars + "hi" word + sentence + paragraph + document = 6
    // Second: t, e, r = 3 new chars + "there" word + sentence + paragraph + document = 7 new
    // Total: 13
    assert_eq!(s2.size(), 13);
}

// ===========================================================================
// Round 6: Diff at word granularity
// ===========================================================================

#[test]
fn diff_same_document_unchanged() {
    let w = test_witnessed();
    let doc = encoding::encode("hello world", &w);
    let changes = diff::diff(&doc, &doc);
    assert_eq!(changes, vec![diff::Change::Unchanged(doc)]);
}

#[test]
fn diff_modified_word() {
    let w = test_witnessed();
    let doc_a = encoding::encode("hello world", &w);
    let doc_b = encoding::encode("hello reed", &w);
    let changes = diff::diff(&doc_a, &doc_b);
    let (_, _, modified, unchanged) = diff::summary(&changes);
    assert!(unchanged > 0);
    assert!(modified > 0);
}

#[test]
fn diff_first_word_unchanged() {
    let w = test_witnessed();
    let doc_a = encoding::encode("hello world", &w);
    let doc_b = encoding::encode("hello reed", &w);
    let changes = diff::diff(&doc_a, &doc_b);
    let has_hello_unchanged = changes.iter().any(|c| match c {
        diff::Change::Unchanged(f) => f.data() == "hello",
        _ => false,
    });
    assert!(has_hello_unchanged);
}

#[test]
fn diff_added_paragraph() {
    let w = test_witnessed();
    let doc_a = encoding::encode("hello", &w);
    let doc_b = encoding::encode("hello\n\nworld", &w);
    let changes = diff::diff(&doc_a, &doc_b);
    assert!(changes.iter().any(|c| matches!(c, diff::Change::Added(_))));
}
