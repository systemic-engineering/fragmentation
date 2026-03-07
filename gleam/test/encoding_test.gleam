import fragmentation
import fragmentation/diff
import fragmentation/encoding
import fragmentation/git
import fragmentation/store
import fragmentation/walk
import gleam/list
import simplifile

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_witnessed() -> fragmentation.Witnessed {
  fragmentation.witnessed(
    fragmentation.author("alex"),
    fragmentation.committer("reed"),
    fragmentation.timestamp("2026-03-01T00:00:00Z"),
    fragmentation.message("test"),
  )
}

// ===========================================================================
// Round 1: Character → Shard
// ===========================================================================

pub fn encode_char_is_shard_test() {
  let w = test_witnessed()
  let result = encoding.encode_char("a", w)
  assert fragmentation.is_shard(result) == True
}

pub fn encode_char_data_test() {
  let w = test_witnessed()
  let result = encoding.encode_char("a", w)
  assert fragmentation.data(result) == "a"
}

pub fn encode_char_label_test() {
  let w = test_witnessed()
  let result = encoding.encode_char("a", w)
  let ref = fragmentation.self_ref(result)
  assert ref.label == "utf8/a"
}

pub fn encode_char_deterministic_test() {
  let w = test_witnessed()
  let a = encoding.encode_char("a", w)
  let b = encoding.encode_char("a", w)
  assert fragmentation.hash_fragment(a) == fragmentation.hash_fragment(b)
}

pub fn encode_char_multibyte_test() {
  let w = test_witnessed()
  let result = encoding.encode_char("é", w)
  assert fragmentation.data(result) == "é"
  let ref = fragmentation.self_ref(result)
  assert ref.label == "utf8/é"
}

pub fn encode_char_different_chars_different_hash_test() {
  let w = test_witnessed()
  let a = encoding.encode_char("a", w)
  let b = encoding.encode_char("b", w)
  assert fragmentation.hash_fragment(a) != fragmentation.hash_fragment(b)
}

// ===========================================================================
// Round 2: Word → Fragment of char Shards
// ===========================================================================

pub fn encode_word_is_fragment_test() {
  let w = test_witnessed()
  let result = encoding.encode_word("hi", w)
  assert fragmentation.is_fragment(result) == True
}

pub fn encode_word_data_test() {
  let w = test_witnessed()
  let result = encoding.encode_word("hi", w)
  assert fragmentation.data(result) == "hi"
}

pub fn encode_word_label_test() {
  let w = test_witnessed()
  let result = encoding.encode_word("hi", w)
  let ref = fragmentation.self_ref(result)
  assert ref.label == "token/hi"
}

pub fn encode_word_children_are_char_shards_test() {
  let w = test_witnessed()
  let result = encoding.encode_word("hi", w)
  let children = fragmentation.children(result)
  assert list.length(children) == 2
  assert list.all(children, fragmentation.is_shard)
}

pub fn encode_word_children_data_test() {
  let w = test_witnessed()
  let result = encoding.encode_word("hi", w)
  let children = fragmentation.children(result)
  let data = list.map(children, fragmentation.data)
  assert data == ["h", "i"]
}

pub fn decode_word_roundtrip_test() {
  let w = test_witnessed()
  let word = encoding.encode_word("hi", w)
  assert encoding.decode(word) == Ok("hi")
}

// ===========================================================================
// Round 3: Paragraph → Fragment of sentence Fragments
// ===========================================================================

pub fn encode_paragraph_is_fragment_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("hi reed", w)
  assert fragmentation.is_fragment(result) == True
}

pub fn encode_paragraph_label_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("hi reed", w)
  let ref = fragmentation.self_ref(result)
  assert ref.label == "paragraph"
}

pub fn encode_paragraph_children_are_sentences_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("hi reed", w)
  let children = fragmentation.children(result)
  // Single sentence (no sentence-ending punctuation followed by space)
  assert list.length(children) == 1
  assert list.all(children, fragmentation.is_fragment)
}

pub fn encode_paragraph_sentence_labels_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("hi reed", w)
  let children = fragmentation.children(result)
  let labels = list.map(children, fn(f) { fragmentation.self_ref(f).label })
  assert labels == ["sentence"]
}

pub fn decode_paragraph_roundtrip_test() {
  let w = test_witnessed()
  let para = encoding.encode_paragraph("hi reed", w)
  assert encoding.decode(para) == Ok("hi reed")
}

pub fn encode_paragraph_filters_empty_words_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("hi  reed", w)
  // Paragraph now contains sentences. Single sentence, which contains words.
  let sentences = fragmentation.children(result)
  assert list.length(sentences) == 1
  let assert Ok(sentence) = list.first(sentences)
  let words = fragmentation.children(sentence)
  assert list.length(words) == 2
}

// ===========================================================================
// Round 3b: Sentence → Fragment of word Fragments
// ===========================================================================

pub fn encode_sentence_is_fragment_test() {
  let w = test_witnessed()
  let result = encoding.encode_sentence("hello world", w)
  assert fragmentation.is_fragment(result) == True
}

pub fn encode_sentence_label_test() {
  let w = test_witnessed()
  let result = encoding.encode_sentence("hello world", w)
  let ref = fragmentation.self_ref(result)
  assert ref.label == "sentence"
}

pub fn encode_sentence_data_test() {
  let w = test_witnessed()
  let result = encoding.encode_sentence("hello world", w)
  assert fragmentation.data(result) == "hello world"
}

pub fn encode_sentence_children_are_words_test() {
  let w = test_witnessed()
  let result = encoding.encode_sentence("hello world", w)
  let children = fragmentation.children(result)
  assert list.length(children) == 2
  let labels = list.map(children, fn(f) { fragmentation.self_ref(f).label })
  assert labels == ["token/hello", "token/world"]
}

pub fn decode_sentence_roundtrip_test() {
  let w = test_witnessed()
  let s = encoding.encode_sentence("hello world", w)
  assert encoding.decode(s) == Ok("hello world")
}

pub fn encode_paragraph_splits_sentences_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("Hello world. How are you?", w)
  let sentences = fragmentation.children(result)
  assert list.length(sentences) == 2
  let assert Ok(first) = list.first(sentences)
  assert fragmentation.data(first) == "Hello world."
}

pub fn encode_paragraph_single_sentence_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("no punctuation here", w)
  let sentences = fragmentation.children(result)
  assert list.length(sentences) == 1
}

pub fn encode_paragraph_exclamation_split_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("Wow! That works.", w)
  let sentences = fragmentation.children(result)
  assert list.length(sentences) == 2
  let assert Ok(first) = list.first(sentences)
  assert fragmentation.data(first) == "Wow!"
}

pub fn encode_paragraph_question_split_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("Really? Yes.", w)
  let sentences = fragmentation.children(result)
  assert list.length(sentences) == 2
}

pub fn encode_paragraph_sentence_has_words_test() {
  let w = test_witnessed()
  let result = encoding.encode_paragraph("Hi Reed. Bye Reed.", w)
  let sentences = fragmentation.children(result)
  let assert Ok(first) = list.first(sentences)
  let words = fragmentation.children(first)
  assert list.length(words) == 2
  let labels = list.map(words, fn(f) { fragmentation.self_ref(f).label })
  assert labels == ["token/Hi", "token/Reed."]
}

// ===========================================================================
// Round 4: Full text round-trip
// ===========================================================================

pub fn encode_is_fragment_test() {
  let w = test_witnessed()
  let result = encoding.encode("Hi Reed.\n\nHow are you?", w)
  assert fragmentation.is_fragment(result) == True
}

pub fn encode_label_is_document_test() {
  let w = test_witnessed()
  let result = encoding.encode("Hi Reed.\n\nHow are you?", w)
  let ref = fragmentation.self_ref(result)
  assert ref.label == "document"
}

pub fn encode_two_paragraphs_test() {
  let w = test_witnessed()
  let result = encoding.encode("Hi Reed.\n\nHow are you?", w)
  let children = fragmentation.children(result)
  assert list.length(children) == 2
}

pub fn encode_paragraph_labels_test() {
  let w = test_witnessed()
  let result = encoding.encode("Hi Reed.\n\nHow are you?", w)
  let children = fragmentation.children(result)
  assert list.all(children, fn(f) {
    fragmentation.self_ref(f).label == "paragraph"
  })
}

pub fn decode_full_roundtrip_test() {
  let w = test_witnessed()
  let text = "Hi Reed.\n\nHow are you?"
  let doc = encoding.encode(text, w)
  assert encoding.decode(doc) == Ok(text)
}

pub fn encode_single_paragraph_test() {
  let w = test_witnessed()
  let result = encoding.encode("just one", w)
  let children = fragmentation.children(result)
  assert list.length(children) == 1
}

pub fn encode_empty_text_test() {
  let w = test_witnessed()
  let result = encoding.encode("", w)
  let children = fragmentation.children(result)
  assert children == []
  // But the document still exists with its data
  assert fragmentation.data(result) == ""
}

// ===========================================================================
// Round 5: Deduplication via ingest + Store
// ===========================================================================

pub fn ingest_returns_root_and_store_test() {
  let w = test_witnessed()
  let s = store.new()
  let #(root, updated) = encoding.ingest("hello", w, s)
  assert fragmentation.is_fragment(root) == True
  assert store.size(updated) > 0
}

pub fn ingest_deduplicates_repeated_words_test() {
  let w = test_witnessed()
  let s = store.new()
  let #(_root, updated) = encoding.ingest("the the the", w, s)
  // 3 unique chars (t, h, e) + 1 word ("the") + 1 sentence + 1 paragraph + 1 document = 7
  assert store.size(updated) == 7
}

pub fn ingest_all_unique_words_test() {
  let w = test_witnessed()
  let s = store.new()
  let #(_root, updated) = encoding.ingest("a b", w, s)
  // chars: a, b = 2, words: "a", "b" = 2, sentence = 1, paragraph = 1, document = 1 = 7
  assert store.size(updated) == 7
}

pub fn ingest_preserves_existing_store_test() {
  let w = test_witnessed()
  let s = store.new()
  let #(_r1, s1) = encoding.ingest("hi", w, s)
  let #(_r2, s2) = encoding.ingest("hi there", w, s1)
  // First: h, i = 2 chars + "hi" word + sentence "hi" + paragraph + document = 6
  // Second: t, e, r = 3 new chars + "there" word + sentence "hi there" + paragraph + document = 7 new
  // Total: 13
  assert store.size(s2) == 13
}

// ===========================================================================
// Round 6: Diff at word granularity
// ===========================================================================

pub fn diff_same_document_unchanged_test() {
  let w = test_witnessed()
  let doc = encoding.encode("hello world", w)
  let changes = diff.diff(doc, doc)
  assert changes == [diff.Unchanged(doc)]
}

pub fn diff_modified_word_test() {
  let w = test_witnessed()
  let doc_a = encoding.encode("hello world", w)
  let doc_b = encoding.encode("hello reed", w)
  let changes = diff.diff(doc_a, doc_b)
  // Root modified, paragraph modified, first word unchanged, second word modified
  let #(_added, _removed, modified, unchanged) = diff.summary(changes)
  assert unchanged > 0
  assert modified > 0
}

pub fn diff_first_word_unchanged_test() {
  let w = test_witnessed()
  let doc_a = encoding.encode("hello world", w)
  let doc_b = encoding.encode("hello reed", w)
  let changes = diff.diff(doc_a, doc_b)
  // "hello" word should appear as Unchanged somewhere
  let has_hello_unchanged =
    list.any(changes, fn(c) {
      case c {
        diff.Unchanged(f) -> fragmentation.data(f) == "hello"
        _ -> False
      }
    })
  assert has_hello_unchanged == True
}

pub fn diff_added_paragraph_test() {
  let w = test_witnessed()
  let doc_a = encoding.encode("hello", w)
  let doc_b = encoding.encode("hello\n\nworld", w)
  let changes = diff.diff(doc_a, doc_b)
  let has_added =
    list.any(changes, fn(c) {
      case c {
        diff.Added(_) -> True
        _ -> False
      }
    })
  assert has_added == True
}

// ===========================================================================
// Round 7: Disk persistence via git.write
// ===========================================================================

pub fn persist_encoding_to_disk_test() {
  let dir = "/tmp/fragmentation_encoding_persist"
  let _ = simplifile.create_directory(dir)
  // Clean slate
  let _ = simplifile.delete(dir)
  let _ = simplifile.create_directory(dir)

  let w = test_witnessed()
  let #(root, s) = encoding.ingest("hi reed", w, store.new())

  // Write every fragment to disk
  let all = walk.collect(root)
  let write_results = list.map(all, fn(f) { git.write(f, dir) })
  assert list.all(write_results, fn(r) {
    case r {
      Ok(_) -> True
      _ -> False
    }
  })

  // File count matches store size (deduped)
  let assert Ok(files) = simplifile.read_directory(dir)
  assert list.length(files) == store.size(s)
}

pub fn persist_deduped_encoding_test() {
  let dir = "/tmp/fragmentation_encoding_dedup_persist"
  let _ = simplifile.delete(dir)
  let _ = simplifile.create_directory(dir)

  let w = test_witnessed()
  let #(root, s) = encoding.ingest("the the", w, store.new())

  // Write all — includes duplicate word nodes in the tree
  let all = walk.collect(root)
  list.each(all, fn(f) {
    let _ = git.write(f, dir)
  })

  // On disk: only unique SHAs survive (idempotent writes)
  let assert Ok(files) = simplifile.read_directory(dir)
  assert list.length(files) == store.size(s)
}
