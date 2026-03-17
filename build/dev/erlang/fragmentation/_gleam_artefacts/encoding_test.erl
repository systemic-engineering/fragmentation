-module(encoding_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/encoding_test.gleam").
-export([encode_char_is_shard_test/0, encode_char_data_test/0, encode_char_label_test/0, encode_char_deterministic_test/0, encode_char_multibyte_test/0, encode_char_different_chars_different_hash_test/0, encode_word_is_fragment_test/0, encode_word_data_test/0, encode_word_label_test/0, encode_word_children_are_char_shards_test/0, encode_word_children_data_test/0, decode_word_roundtrip_test/0, encode_paragraph_is_fragment_test/0, encode_paragraph_label_test/0, encode_paragraph_children_are_sentences_test/0, encode_paragraph_sentence_labels_test/0, decode_paragraph_roundtrip_test/0, encode_paragraph_filters_empty_words_test/0, encode_sentence_is_fragment_test/0, encode_sentence_label_test/0, encode_sentence_data_test/0, encode_sentence_children_are_words_test/0, decode_sentence_roundtrip_test/0, encode_paragraph_splits_sentences_test/0, encode_paragraph_single_sentence_test/0, encode_paragraph_exclamation_split_test/0, encode_paragraph_question_split_test/0, encode_paragraph_sentence_has_words_test/0, encode_is_fragment_test/0, encode_label_is_document_test/0, encode_two_paragraphs_test/0, encode_paragraph_labels_test/0, decode_full_roundtrip_test/0, encode_single_paragraph_test/0, encode_empty_text_test/0, ingest_returns_root_and_store_test/0, ingest_deduplicates_repeated_words_test/0, ingest_all_unique_words_test/0, ingest_preserves_existing_store_test/0, diff_same_document_unchanged_test/0, diff_modified_word_test/0, diff_first_word_unchanged_test/0, diff_added_paragraph_test/0, persist_encoding_to_disk_test/0, persist_deduped_encoding_test/0]).

-file("test/encoding_test.gleam", 14).
-spec test_witnessed() -> fragmentation:witnessed().
test_witnessed() ->
    fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01T00:00:00Z"/utf8>>),
        fragmentation:message(<<"test"/utf8>>)
    ).

-file("test/encoding_test.gleam", 27).
-spec encode_char_is_shard_test() -> nil.
encode_char_is_shard_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_char(<<"a"/utf8>>, W),
    _assert_subject = fragmentation:is_shard(Result),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_char_is_shard_test"/utf8>>,
                line => 30,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 912,
                    'end' => 942
                    },
                right => #{kind => literal,
                    value => true,
                    start => 946,
                    'end' => 950
                    },
                start => 905,
                'end' => 950,
                expression_start => 912})
    end.

-file("test/encoding_test.gleam", 33).
-spec encode_char_data_test() -> nil.
encode_char_data_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_char(<<"a"/utf8>>, W),
    _assert_subject = fragmentation:data(Result),
    _assert_subject@1 = <<"a"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_char_data_test"/utf8>>,
                line => 36,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 1067,
                    'end' => 1093
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 1097,
                    'end' => 1100
                    },
                start => 1060,
                'end' => 1100,
                expression_start => 1067})
    end.

-file("test/encoding_test.gleam", 39).
-spec encode_char_label_test() -> nil.
encode_char_label_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_char(<<"a"/utf8>>, W),
    Ref = fragmentation:self_ref(Result),
    _assert_subject = erlang:element(3, Ref),
    _assert_subject@1 = <<"utf8/a"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_char_label_test"/utf8>>,
                line => 43,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 1261,
                    'end' => 1270
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 1274,
                    'end' => 1282
                    },
                start => 1254,
                'end' => 1282,
                expression_start => 1261})
    end.

-file("test/encoding_test.gleam", 46).
-spec encode_char_deterministic_test() -> nil.
encode_char_deterministic_test() ->
    W = test_witnessed(),
    A = fragmentation@encoding:encode_char(<<"a"/utf8>>, W),
    B = fragmentation@encoding:encode_char(<<"a"/utf8>>, W),
    _assert_subject = fragmentation:hash_fragment(A),
    _assert_subject@1 = fragmentation:hash_fragment(B),
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_char_deterministic_test"/utf8>>,
                line => 50,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 1442,
                    'end' => 1472
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 1476,
                    'end' => 1506
                    },
                start => 1435,
                'end' => 1506,
                expression_start => 1442})
    end.

-file("test/encoding_test.gleam", 53).
-spec encode_char_multibyte_test() -> nil.
encode_char_multibyte_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_char(<<"é"/utf8>>, W),
    _assert_subject = fragmentation:data(Result),
    _assert_subject@1 = <<"é"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_char_multibyte_test"/utf8>>,
                line => 56,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 1629,
                    'end' => 1655
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 1659,
                    'end' => 1663
                    },
                start => 1622,
                'end' => 1663,
                expression_start => 1629})
    end,
    Ref = fragmentation:self_ref(Result),
    _assert_subject@2 = erlang:element(3, Ref),
    _assert_subject@3 = <<"utf8/é"/utf8>>,
    case _assert_subject@2 =:= _assert_subject@3 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_char_multibyte_test"/utf8>>,
                line => 58,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@2,
                    start => 1716,
                    'end' => 1725
                    },
                right => #{kind => literal,
                    value => _assert_subject@3,
                    start => 1729,
                    'end' => 1738
                    },
                start => 1709,
                'end' => 1738,
                expression_start => 1716})
    end.

-file("test/encoding_test.gleam", 61).
-spec encode_char_different_chars_different_hash_test() -> nil.
encode_char_different_chars_different_hash_test() ->
    W = test_witnessed(),
    A = fragmentation@encoding:encode_char(<<"a"/utf8>>, W),
    B = fragmentation@encoding:encode_char(<<"b"/utf8>>, W),
    _assert_subject = fragmentation:hash_fragment(A),
    _assert_subject@1 = fragmentation:hash_fragment(B),
    case _assert_subject /= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_char_different_chars_different_hash_test"/utf8>>,
                line => 65,
                kind => binary_operator,
                operator => '!=',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 1915,
                    'end' => 1945
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 1949,
                    'end' => 1979
                    },
                start => 1908,
                'end' => 1979,
                expression_start => 1915})
    end.

-file("test/encoding_test.gleam", 72).
-spec encode_word_is_fragment_test() -> nil.
encode_word_is_fragment_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_word(<<"hi"/utf8>>, W),
    _assert_subject = fragmentation:is_fragment(Result),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_word_is_fragment_test"/utf8>>,
                line => 75,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 2308,
                    'end' => 2341
                    },
                right => #{kind => literal,
                    value => true,
                    start => 2345,
                    'end' => 2349
                    },
                start => 2301,
                'end' => 2349,
                expression_start => 2308})
    end.

-file("test/encoding_test.gleam", 78).
-spec encode_word_data_test() -> nil.
encode_word_data_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_word(<<"hi"/utf8>>, W),
    _assert_subject = fragmentation:data(Result),
    _assert_subject@1 = <<"hi"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_word_data_test"/utf8>>,
                line => 81,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 2467,
                    'end' => 2493
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 2497,
                    'end' => 2501
                    },
                start => 2460,
                'end' => 2501,
                expression_start => 2467})
    end.

-file("test/encoding_test.gleam", 84).
-spec encode_word_label_test() -> nil.
encode_word_label_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_word(<<"hi"/utf8>>, W),
    Ref = fragmentation:self_ref(Result),
    _assert_subject = erlang:element(3, Ref),
    _assert_subject@1 = <<"token/hi"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_word_label_test"/utf8>>,
                line => 88,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 2663,
                    'end' => 2672
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 2676,
                    'end' => 2686
                    },
                start => 2656,
                'end' => 2686,
                expression_start => 2663})
    end.

-file("test/encoding_test.gleam", 91).
-spec encode_word_children_are_char_shards_test() -> nil.
encode_word_children_are_char_shards_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_word(<<"hi"/utf8>>, W),
    Children = fragmentation:children(Result),
    _assert_subject = erlang:length(Children),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_word_children_are_char_shards_test"/utf8>>,
                line => 95,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 2872,
                    'end' => 2893
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 2897,
                    'end' => 2898
                    },
                start => 2865,
                'end' => 2898,
                expression_start => 2872})
    end,
    _assert_subject@2 = fun fragmentation:is_shard/1,
    case gleam@list:all(Children, _assert_subject@2) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_word_children_are_char_shards_test"/utf8>>,
                line => 96,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => Children,
                        start => 2917,
                        'end' => 2925
                        }, #{kind => expression,
                        value => _assert_subject@2,
                        start => 2927,
                        'end' => 2949
                        }],
                start => 2901,
                'end' => 2950,
                expression_start => 2908})
    end.

-file("test/encoding_test.gleam", 99).
-spec encode_word_children_data_test() -> nil.
encode_word_children_data_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_word(<<"hi"/utf8>>, W),
    Children = fragmentation:children(Result),
    Data = gleam@list:map(Children, fun fragmentation:data/1),
    _assert_subject = [<<"h"/utf8>>, <<"i"/utf8>>],
    case Data =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_word_children_data_test"/utf8>>,
                line => 104,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Data,
                    start => 3177,
                    'end' => 3181
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 3185,
                    'end' => 3195
                    },
                start => 3170,
                'end' => 3195,
                expression_start => 3177})
    end.

-file("test/encoding_test.gleam", 107).
-spec decode_word_roundtrip_test() -> nil.
decode_word_roundtrip_test() ->
    W = test_witnessed(),
    Word = fragmentation@encoding:encode_word(<<"hi"/utf8>>, W),
    _assert_subject = fragmentation@encoding:decode(Word),
    _assert_subject@1 = {ok, <<"hi"/utf8>>},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"decode_word_roundtrip_test"/utf8>>,
                line => 110,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 3316,
                    'end' => 3337
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 3341,
                    'end' => 3349
                    },
                start => 3309,
                'end' => 3349,
                expression_start => 3316})
    end.

-file("test/encoding_test.gleam", 117).
-spec encode_paragraph_is_fragment_test() -> nil.
encode_paragraph_is_fragment_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(<<"hi reed"/utf8>>, W),
    _assert_subject = fragmentation:is_fragment(Result),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_is_fragment_test"/utf8>>,
                line => 120,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 3705,
                    'end' => 3738
                    },
                right => #{kind => literal,
                    value => true,
                    start => 3742,
                    'end' => 3746
                    },
                start => 3698,
                'end' => 3746,
                expression_start => 3705})
    end.

-file("test/encoding_test.gleam", 123).
-spec encode_paragraph_label_test() -> nil.
encode_paragraph_label_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(<<"hi reed"/utf8>>, W),
    Ref = fragmentation:self_ref(Result),
    _assert_subject = erlang:element(3, Ref),
    _assert_subject@1 = <<"paragraph"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_label_test"/utf8>>,
                line => 127,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 3923,
                    'end' => 3932
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 3936,
                    'end' => 3947
                    },
                start => 3916,
                'end' => 3947,
                expression_start => 3923})
    end.

-file("test/encoding_test.gleam", 130).
-spec encode_paragraph_children_are_sentences_test() -> nil.
encode_paragraph_children_are_sentences_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(<<"hi reed"/utf8>>, W),
    Children = fragmentation:children(Result),
    _assert_subject = erlang:length(Children),
    _assert_subject@1 = 1,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_children_are_sentences_test"/utf8>>,
                line => 135,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 4218,
                    'end' => 4239
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 4243,
                    'end' => 4244
                    },
                start => 4211,
                'end' => 4244,
                expression_start => 4218})
    end,
    _assert_subject@2 = fun fragmentation:is_fragment/1,
    case gleam@list:all(Children, _assert_subject@2) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_children_are_sentences_test"/utf8>>,
                line => 136,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => Children,
                        start => 4263,
                        'end' => 4271
                        }, #{kind => expression,
                        value => _assert_subject@2,
                        start => 4273,
                        'end' => 4298
                        }],
                start => 4247,
                'end' => 4299,
                expression_start => 4254})
    end.

-file("test/encoding_test.gleam", 139).
-spec encode_paragraph_sentence_labels_test() -> nil.
encode_paragraph_sentence_labels_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(<<"hi reed"/utf8>>, W),
    Children = fragmentation:children(Result),
    Labels = gleam@list:map(
        Children,
        fun(F) -> erlang:element(3, fragmentation:self_ref(F)) end
    ),
    _assert_subject = [<<"sentence"/utf8>>],
    case Labels =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_sentence_labels_test"/utf8>>,
                line => 144,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Labels,
                    start => 4568,
                    'end' => 4574
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 4578,
                    'end' => 4590
                    },
                start => 4561,
                'end' => 4590,
                expression_start => 4568})
    end.

-file("test/encoding_test.gleam", 147).
-spec decode_paragraph_roundtrip_test() -> nil.
decode_paragraph_roundtrip_test() ->
    W = test_witnessed(),
    Para = fragmentation@encoding:encode_paragraph(<<"hi reed"/utf8>>, W),
    _assert_subject = fragmentation@encoding:decode(Para),
    _assert_subject@1 = {ok, <<"hi reed"/utf8>>},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"decode_paragraph_roundtrip_test"/utf8>>,
                line => 150,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 4726,
                    'end' => 4747
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 4751,
                    'end' => 4764
                    },
                start => 4719,
                'end' => 4764,
                expression_start => 4726})
    end.

-file("test/encoding_test.gleam", 153).
-spec encode_paragraph_filters_empty_words_test() -> nil.
encode_paragraph_filters_empty_words_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(<<"hi  reed"/utf8>>, W),
    Sentences = fragmentation:children(Result),
    _assert_subject = erlang:length(Sentences),
    _assert_subject@1 = 1,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_filters_empty_words_test"/utf8>>,
                line => 158,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5040,
                    'end' => 5062
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 5066,
                    'end' => 5067
                    },
                start => 5033,
                'end' => 5067,
                expression_start => 5040})
    end,
    Sentence@1 = case gleam@list:first(Sentences) of
        {ok, Sentence} -> Sentence;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"encoding_test"/utf8>>,
                        function => <<"encode_paragraph_filters_empty_words_test"/utf8>>,
                        line => 159,
                        value => _assert_fail,
                        start => 5070,
                        'end' => 5117,
                        pattern_start => 5081,
                        pattern_end => 5093})
    end,
    Words = fragmentation:children(Sentence@1),
    _assert_subject@2 = erlang:length(Words),
    _assert_subject@3 = 2,
    case _assert_subject@2 =:= _assert_subject@3 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_filters_empty_words_test"/utf8>>,
                line => 161,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@2,
                    start => 5174,
                    'end' => 5192
                    },
                right => #{kind => literal,
                    value => _assert_subject@3,
                    start => 5196,
                    'end' => 5197
                    },
                start => 5167,
                'end' => 5197,
                expression_start => 5174})
    end.

-file("test/encoding_test.gleam", 168).
-spec encode_sentence_is_fragment_test() -> nil.
encode_sentence_is_fragment_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_sentence(<<"hello world"/utf8>>, W),
    _assert_subject = fragmentation:is_fragment(Result),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_sentence_is_fragment_test"/utf8>>,
                line => 171,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5551,
                    'end' => 5584
                    },
                right => #{kind => literal,
                    value => true,
                    start => 5588,
                    'end' => 5592
                    },
                start => 5544,
                'end' => 5592,
                expression_start => 5551})
    end.

-file("test/encoding_test.gleam", 174).
-spec encode_sentence_label_test() -> nil.
encode_sentence_label_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_sentence(<<"hello world"/utf8>>, W),
    Ref = fragmentation:self_ref(Result),
    _assert_subject = erlang:element(3, Ref),
    _assert_subject@1 = <<"sentence"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_sentence_label_test"/utf8>>,
                line => 178,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5771,
                    'end' => 5780
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 5784,
                    'end' => 5794
                    },
                start => 5764,
                'end' => 5794,
                expression_start => 5771})
    end.

-file("test/encoding_test.gleam", 181).
-spec encode_sentence_data_test() -> nil.
encode_sentence_data_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_sentence(<<"hello world"/utf8>>, W),
    _assert_subject = fragmentation:data(Result),
    _assert_subject@1 = <<"hello world"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_sentence_data_test"/utf8>>,
                line => 184,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5929,
                    'end' => 5955
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 5959,
                    'end' => 5972
                    },
                start => 5922,
                'end' => 5972,
                expression_start => 5929})
    end.

-file("test/encoding_test.gleam", 187).
-spec encode_sentence_children_are_words_test() -> nil.
encode_sentence_children_are_words_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_sentence(<<"hello world"/utf8>>, W),
    Children = fragmentation:children(Result),
    _assert_subject = erlang:length(Children),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_sentence_children_are_words_test"/utf8>>,
                line => 191,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 6169,
                    'end' => 6190
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 6194,
                    'end' => 6195
                    },
                start => 6162,
                'end' => 6195,
                expression_start => 6169})
    end,
    Labels = gleam@list:map(
        Children,
        fun(F) -> erlang:element(3, fragmentation:self_ref(F)) end
    ),
    _assert_subject@2 = [<<"token/hello"/utf8>>, <<"token/world"/utf8>>],
    case Labels =:= _assert_subject@2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_sentence_children_are_words_test"/utf8>>,
                line => 193,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Labels,
                    start => 6282,
                    'end' => 6288
                    },
                right => #{kind => literal,
                    value => _assert_subject@2,
                    start => 6292,
                    'end' => 6322
                    },
                start => 6275,
                'end' => 6322,
                expression_start => 6282})
    end.

-file("test/encoding_test.gleam", 196).
-spec decode_sentence_roundtrip_test() -> nil.
decode_sentence_roundtrip_test() ->
    W = test_witnessed(),
    S = fragmentation@encoding:encode_sentence(<<"hello world"/utf8>>, W),
    _assert_subject = fragmentation@encoding:decode(S),
    _assert_subject@1 = {ok, <<"hello world"/utf8>>},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"decode_sentence_roundtrip_test"/utf8>>,
                line => 199,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 6457,
                    'end' => 6475
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 6479,
                    'end' => 6496
                    },
                start => 6450,
                'end' => 6496,
                expression_start => 6457})
    end.

-file("test/encoding_test.gleam", 202).
-spec encode_paragraph_splits_sentences_test() -> nil.
encode_paragraph_splits_sentences_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(
        <<"Hello world. How are you?"/utf8>>,
        W
    ),
    Sentences = fragmentation:children(Result),
    _assert_subject = erlang:length(Sentences),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_splits_sentences_test"/utf8>>,
                line => 206,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 6708,
                    'end' => 6730
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 6734,
                    'end' => 6735
                    },
                start => 6701,
                'end' => 6735,
                expression_start => 6708})
    end,
    First@1 = case gleam@list:first(Sentences) of
        {ok, First} -> First;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"encoding_test"/utf8>>,
                        function => <<"encode_paragraph_splits_sentences_test"/utf8>>,
                        line => 207,
                        value => _assert_fail,
                        start => 6738,
                        'end' => 6782,
                        pattern_start => 6749,
                        pattern_end => 6758})
    end,
    _assert_subject@2 = fragmentation:data(First@1),
    _assert_subject@3 = <<"Hello world."/utf8>>,
    case _assert_subject@2 =:= _assert_subject@3 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_splits_sentences_test"/utf8>>,
                line => 208,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@2,
                    start => 6792,
                    'end' => 6817
                    },
                right => #{kind => literal,
                    value => _assert_subject@3,
                    start => 6821,
                    'end' => 6835
                    },
                start => 6785,
                'end' => 6835,
                expression_start => 6792})
    end.

-file("test/encoding_test.gleam", 211).
-spec encode_paragraph_single_sentence_test() -> nil.
encode_paragraph_single_sentence_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(
        <<"no punctuation here"/utf8>>,
        W
    ),
    Sentences = fragmentation:children(Result),
    _assert_subject = erlang:length(Sentences),
    _assert_subject@1 = 1,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_single_sentence_test"/utf8>>,
                line => 215,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 7040,
                    'end' => 7062
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 7066,
                    'end' => 7067
                    },
                start => 7033,
                'end' => 7067,
                expression_start => 7040})
    end.

-file("test/encoding_test.gleam", 218).
-spec encode_paragraph_exclamation_split_test() -> nil.
encode_paragraph_exclamation_split_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(
        <<"Wow! That works."/utf8>>,
        W
    ),
    Sentences = fragmentation:children(Result),
    _assert_subject = erlang:length(Sentences),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_exclamation_split_test"/utf8>>,
                line => 222,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 7271,
                    'end' => 7293
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 7297,
                    'end' => 7298
                    },
                start => 7264,
                'end' => 7298,
                expression_start => 7271})
    end,
    First@1 = case gleam@list:first(Sentences) of
        {ok, First} -> First;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"encoding_test"/utf8>>,
                        function => <<"encode_paragraph_exclamation_split_test"/utf8>>,
                        line => 223,
                        value => _assert_fail,
                        start => 7301,
                        'end' => 7345,
                        pattern_start => 7312,
                        pattern_end => 7321})
    end,
    _assert_subject@2 = fragmentation:data(First@1),
    _assert_subject@3 = <<"Wow!"/utf8>>,
    case _assert_subject@2 =:= _assert_subject@3 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_exclamation_split_test"/utf8>>,
                line => 224,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@2,
                    start => 7355,
                    'end' => 7380
                    },
                right => #{kind => literal,
                    value => _assert_subject@3,
                    start => 7384,
                    'end' => 7390
                    },
                start => 7348,
                'end' => 7390,
                expression_start => 7355})
    end.

-file("test/encoding_test.gleam", 227).
-spec encode_paragraph_question_split_test() -> nil.
encode_paragraph_question_split_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(<<"Really? Yes."/utf8>>, W),
    Sentences = fragmentation:children(Result),
    _assert_subject = erlang:length(Sentences),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_question_split_test"/utf8>>,
                line => 231,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 7587,
                    'end' => 7609
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 7613,
                    'end' => 7614
                    },
                start => 7580,
                'end' => 7614,
                expression_start => 7587})
    end.

-file("test/encoding_test.gleam", 234).
-spec encode_paragraph_sentence_has_words_test() -> nil.
encode_paragraph_sentence_has_words_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode_paragraph(
        <<"Hi Reed. Bye Reed."/utf8>>,
        W
    ),
    Sentences = fragmentation:children(Result),
    First@1 = case gleam@list:first(Sentences) of
        {ok, First} -> First;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"encoding_test"/utf8>>,
                        function => <<"encode_paragraph_sentence_has_words_test"/utf8>>,
                        line => 238,
                        value => _assert_fail,
                        start => 7814,
                        'end' => 7858,
                        pattern_start => 7825,
                        pattern_end => 7834})
    end,
    Words = fragmentation:children(First@1),
    _assert_subject = erlang:length(Words),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_sentence_has_words_test"/utf8>>,
                line => 240,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 7912,
                    'end' => 7930
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 7934,
                    'end' => 7935
                    },
                start => 7905,
                'end' => 7935,
                expression_start => 7912})
    end,
    Labels = gleam@list:map(
        Words,
        fun(F) -> erlang:element(3, fragmentation:self_ref(F)) end
    ),
    _assert_subject@2 = [<<"token/Hi"/utf8>>, <<"token/Reed."/utf8>>],
    case Labels =:= _assert_subject@2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_sentence_has_words_test"/utf8>>,
                line => 242,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Labels,
                    start => 8019,
                    'end' => 8025
                    },
                right => #{kind => literal,
                    value => _assert_subject@2,
                    start => 8029,
                    'end' => 8056
                    },
                start => 8012,
                'end' => 8056,
                expression_start => 8019})
    end.

-file("test/encoding_test.gleam", 249).
-spec encode_is_fragment_test() -> nil.
encode_is_fragment_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode(
        <<"Hi Reed.\n\nHow are you?"/utf8>>,
        W
    ),
    _assert_subject = fragmentation:is_fragment(Result),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_is_fragment_test"/utf8>>,
                line => 252,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8385,
                    'end' => 8418
                    },
                right => #{kind => literal,
                    value => true,
                    start => 8422,
                    'end' => 8426
                    },
                start => 8378,
                'end' => 8426,
                expression_start => 8385})
    end.

-file("test/encoding_test.gleam", 255).
-spec encode_label_is_document_test() -> nil.
encode_label_is_document_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode(
        <<"Hi Reed.\n\nHow are you?"/utf8>>,
        W
    ),
    Ref = fragmentation:self_ref(Result),
    _assert_subject = erlang:element(3, Ref),
    _assert_subject@1 = <<"document"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_label_is_document_test"/utf8>>,
                line => 259,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8612,
                    'end' => 8621
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 8625,
                    'end' => 8635
                    },
                start => 8605,
                'end' => 8635,
                expression_start => 8612})
    end.

-file("test/encoding_test.gleam", 262).
-spec encode_two_paragraphs_test() -> nil.
encode_two_paragraphs_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode(
        <<"Hi Reed.\n\nHow are you?"/utf8>>,
        W
    ),
    Children = fragmentation:children(Result),
    _assert_subject = erlang:length(Children),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_two_paragraphs_test"/utf8>>,
                line => 266,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8823,
                    'end' => 8844
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 8848,
                    'end' => 8849
                    },
                start => 8816,
                'end' => 8849,
                expression_start => 8823})
    end.

-file("test/encoding_test.gleam", 269).
-spec encode_paragraph_labels_test() -> nil.
encode_paragraph_labels_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode(
        <<"Hi Reed.\n\nHow are you?"/utf8>>,
        W
    ),
    Children = fragmentation:children(Result),
    _assert_subject = fun(F) ->
        erlang:element(3, fragmentation:self_ref(F)) =:= <<"paragraph"/utf8>>
    end,
    case gleam@list:all(Children, _assert_subject) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_paragraph_labels_test"/utf8>>,
                line => 273,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => Children,
                        start => 9048,
                        'end' => 9056
                        }, #{kind => expression,
                        value => _assert_subject,
                        start => 9058,
                        'end' => 9120
                        }],
                start => 9032,
                'end' => 9121,
                expression_start => 9039})
    end.

-file("test/encoding_test.gleam", 278).
-spec decode_full_roundtrip_test() -> nil.
decode_full_roundtrip_test() ->
    W = test_witnessed(),
    Text = <<"Hi Reed.\n\nHow are you?"/utf8>>,
    Doc = fragmentation@encoding:encode(Text, W),
    _assert_subject = fragmentation@encoding:decode(Doc),
    _assert_subject@1 = {ok, Text},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"decode_full_roundtrip_test"/utf8>>,
                line => 282,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 9276,
                    'end' => 9296
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 9300,
                    'end' => 9308
                    },
                start => 9269,
                'end' => 9308,
                expression_start => 9276})
    end.

-file("test/encoding_test.gleam", 285).
-spec encode_single_paragraph_test() -> nil.
encode_single_paragraph_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode(<<"just one"/utf8>>, W),
    Children = fragmentation:children(Result),
    _assert_subject = erlang:length(Children),
    _assert_subject@1 = 1,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_single_paragraph_test"/utf8>>,
                line => 289,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 9482,
                    'end' => 9503
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 9507,
                    'end' => 9508
                    },
                start => 9475,
                'end' => 9508,
                expression_start => 9482})
    end.

-file("test/encoding_test.gleam", 292).
-spec encode_empty_text_test() -> nil.
encode_empty_text_test() ->
    W = test_witnessed(),
    Result = fragmentation@encoding:encode(<<""/utf8>>, W),
    Children = fragmentation:children(Result),
    _assert_subject = [],
    case Children =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_empty_text_test"/utf8>>,
                line => 296,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Children,
                    start => 9668,
                    'end' => 9676
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 9680,
                    'end' => 9682
                    },
                start => 9661,
                'end' => 9682,
                expression_start => 9668})
    end,
    _assert_subject@1 = fragmentation:data(Result),
    _assert_subject@2 = <<""/utf8>>,
    case _assert_subject@1 =:= _assert_subject@2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"encode_empty_text_test"/utf8>>,
                line => 298,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@1,
                    start => 9741,
                    'end' => 9767
                    },
                right => #{kind => literal,
                    value => _assert_subject@2,
                    start => 9771,
                    'end' => 9773
                    },
                start => 9734,
                'end' => 9773,
                expression_start => 9741})
    end.

-file("test/encoding_test.gleam", 305).
-spec ingest_returns_root_and_store_test() -> nil.
ingest_returns_root_and_store_test() ->
    W = test_witnessed(),
    S = fragmentation@store:new(),
    {Root, Updated} = fragmentation@encoding:ingest(<<"hello"/utf8>>, W, S),
    _assert_subject = fragmentation:is_fragment(Root),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"ingest_returns_root_and_store_test"/utf8>>,
                line => 309,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 10141,
                    'end' => 10172
                    },
                right => #{kind => literal,
                    value => true,
                    start => 10176,
                    'end' => 10180
                    },
                start => 10134,
                'end' => 10180,
                expression_start => 10141})
    end,
    _assert_subject@1 = fragmentation@store:size(Updated),
    _assert_subject@2 = 0,
    case _assert_subject@1 > _assert_subject@2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"ingest_returns_root_and_store_test"/utf8>>,
                line => 310,
                kind => binary_operator,
                operator => '>',
                left => #{kind => expression,
                    value => _assert_subject@1,
                    start => 10190,
                    'end' => 10209
                    },
                right => #{kind => literal,
                    value => _assert_subject@2,
                    start => 10212,
                    'end' => 10213
                    },
                start => 10183,
                'end' => 10213,
                expression_start => 10190})
    end.

-file("test/encoding_test.gleam", 313).
-spec ingest_deduplicates_repeated_words_test() -> nil.
ingest_deduplicates_repeated_words_test() ->
    W = test_witnessed(),
    S = fragmentation@store:new(),
    {_, Updated} = fragmentation@encoding:ingest(<<"the the the"/utf8>>, W, S),
    _assert_subject = fragmentation@store:size(Updated),
    _assert_subject@1 = 7,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"ingest_deduplicates_repeated_words_test"/utf8>>,
                line => 318,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 10480,
                    'end' => 10499
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 10503,
                    'end' => 10504
                    },
                start => 10473,
                'end' => 10504,
                expression_start => 10480})
    end.

-file("test/encoding_test.gleam", 321).
-spec ingest_all_unique_words_test() -> nil.
ingest_all_unique_words_test() ->
    W = test_witnessed(),
    S = fragmentation@store:new(),
    {_, Updated} = fragmentation@encoding:ingest(<<"a b"/utf8>>, W, S),
    _assert_subject = fragmentation@store:size(Updated),
    _assert_subject@1 = 7,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"ingest_all_unique_words_test"/utf8>>,
                line => 326,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 10750,
                    'end' => 10769
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 10773,
                    'end' => 10774
                    },
                start => 10743,
                'end' => 10774,
                expression_start => 10750})
    end.

-file("test/encoding_test.gleam", 329).
-spec ingest_preserves_existing_store_test() -> nil.
ingest_preserves_existing_store_test() ->
    W = test_witnessed(),
    S = fragmentation@store:new(),
    {_, S1} = fragmentation@encoding:ingest(<<"hi"/utf8>>, W, S),
    {_, S2} = fragmentation@encoding:ingest(<<"hi there"/utf8>>, W, S1),
    _assert_subject = fragmentation@store:size(S2),
    _assert_subject@1 = 13,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"ingest_preserves_existing_store_test"/utf8>>,
                line => 337,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 11185,
                    'end' => 11199
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 11203,
                    'end' => 11205
                    },
                start => 11178,
                'end' => 11205,
                expression_start => 11185})
    end.

-file("test/encoding_test.gleam", 344).
-spec diff_same_document_unchanged_test() -> nil.
diff_same_document_unchanged_test() ->
    W = test_witnessed(),
    Doc = fragmentation@encoding:encode(<<"hello world"/utf8>>, W),
    Changes = fragmentation@diff:diff(Doc, Doc),
    _assert_subject = [{unchanged, Doc}],
    case Changes =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"diff_same_document_unchanged_test"/utf8>>,
                line => 348,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Changes,
                    start => 11568,
                    'end' => 11575
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 11579,
                    'end' => 11600
                    },
                start => 11561,
                'end' => 11600,
                expression_start => 11568})
    end.

-file("test/encoding_test.gleam", 351).
-spec diff_modified_word_test() -> nil.
diff_modified_word_test() ->
    W = test_witnessed(),
    Doc_a = fragmentation@encoding:encode(<<"hello world"/utf8>>, W),
    Doc_b = fragmentation@encoding:encode(<<"hello reed"/utf8>>, W),
    Changes = fragmentation@diff:diff(Doc_a, Doc_b),
    {_, _, Modified, Unchanged} = fragmentation@diff:summary(Changes),
    _assert_subject = 0,
    case Unchanged > _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"diff_modified_word_test"/utf8>>,
                line => 358,
                kind => binary_operator,
                operator => '>',
                left => #{kind => expression,
                    value => Unchanged,
                    start => 11964,
                    'end' => 11973
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 11976,
                    'end' => 11977
                    },
                start => 11957,
                'end' => 11977,
                expression_start => 11964})
    end,
    _assert_subject@1 = 0,
    case Modified > _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"diff_modified_word_test"/utf8>>,
                line => 359,
                kind => binary_operator,
                operator => '>',
                left => #{kind => expression,
                    value => Modified,
                    start => 11987,
                    'end' => 11995
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 11998,
                    'end' => 11999
                    },
                start => 11980,
                'end' => 11999,
                expression_start => 11987})
    end.

-file("test/encoding_test.gleam", 362).
-spec diff_first_word_unchanged_test() -> nil.
diff_first_word_unchanged_test() ->
    W = test_witnessed(),
    Doc_a = fragmentation@encoding:encode(<<"hello world"/utf8>>, W),
    Doc_b = fragmentation@encoding:encode(<<"hello reed"/utf8>>, W),
    Changes = fragmentation@diff:diff(Doc_a, Doc_b),
    Has_hello_unchanged = gleam@list:any(Changes, fun(C) -> case C of
                {unchanged, F} ->
                    fragmentation:data(F) =:= <<"hello"/utf8>>;

                _ ->
                    false
            end end),
    case Has_hello_unchanged =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"diff_first_word_unchanged_test"/utf8>>,
                line => 375,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Has_hello_unchanged,
                    start => 12440,
                    'end' => 12459
                    },
                right => #{kind => literal,
                    value => true,
                    start => 12463,
                    'end' => 12467
                    },
                start => 12433,
                'end' => 12467,
                expression_start => 12440})
    end.

-file("test/encoding_test.gleam", 378).
-spec diff_added_paragraph_test() -> nil.
diff_added_paragraph_test() ->
    W = test_witnessed(),
    Doc_a = fragmentation@encoding:encode(<<"hello"/utf8>>, W),
    Doc_b = fragmentation@encoding:encode(<<"hello\n\nworld"/utf8>>, W),
    Changes = fragmentation@diff:diff(Doc_a, Doc_b),
    Has_added = gleam@list:any(Changes, fun(C) -> case C of
                {added, _} ->
                    true;

                _ ->
                    false
            end end),
    case Has_added =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"diff_added_paragraph_test"/utf8>>,
                line => 390,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Has_added,
                    start => 12804,
                    'end' => 12813
                    },
                right => #{kind => literal,
                    value => true,
                    start => 12817,
                    'end' => 12821
                    },
                start => 12797,
                'end' => 12821,
                expression_start => 12804})
    end.

-file("test/encoding_test.gleam", 397).
-spec persist_encoding_to_disk_test() -> nil.
persist_encoding_to_disk_test() ->
    Dir = <<"/tmp/fragmentation_encoding_persist"/utf8>>,
    _ = simplifile_erl:create_directory(Dir),
    _ = simplifile_erl:delete(Dir),
    _ = simplifile_erl:create_directory(Dir),
    W = test_witnessed(),
    {Root, S} = fragmentation@encoding:ingest(
        <<"hi reed"/utf8>>,
        W,
        fragmentation@store:new()
    ),
    All = fragmentation@walk:collect(Root),
    Write_results = gleam@list:map(
        All,
        fun(F) -> fragmentation@git:write(F, Dir) end
    ),
    _assert_subject = fun(R) -> case R of
            {ok, _} ->
                true;

            _ ->
                false
        end end,
    case gleam@list:all(Write_results, _assert_subject) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"persist_encoding_to_disk_test"/utf8>>,
                line => 410,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => Write_results,
                        start => 13493,
                        'end' => 13506
                        }, #{kind => expression,
                        value => _assert_subject,
                        start => 13508,
                        'end' => 13575
                        }],
                start => 13477,
                'end' => 13576,
                expression_start => 13484})
    end,
    Files@1 = case simplifile_erl:read_directory(Dir) of
        {ok, Files} -> Files;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"encoding_test"/utf8>>,
                        function => <<"persist_encoding_to_disk_test"/utf8>>,
                        line => 418,
                        value => _assert_fail,
                        start => 13625,
                        'end' => 13678,
                        pattern_start => 13636,
                        pattern_end => 13645})
    end,
    _assert_subject@1 = erlang:length(Files@1),
    _assert_subject@2 = fragmentation@store:size(S),
    case _assert_subject@1 =:= _assert_subject@2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"persist_encoding_to_disk_test"/utf8>>,
                line => 419,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@1,
                    start => 13688,
                    'end' => 13706
                    },
                right => #{kind => expression,
                    value => _assert_subject@2,
                    start => 13710,
                    'end' => 13723
                    },
                start => 13681,
                'end' => 13723,
                expression_start => 13688})
    end.

-file("test/encoding_test.gleam", 422).
-spec persist_deduped_encoding_test() -> nil.
persist_deduped_encoding_test() ->
    Dir = <<"/tmp/fragmentation_encoding_dedup_persist"/utf8>>,
    _ = simplifile_erl:delete(Dir),
    _ = simplifile_erl:create_directory(Dir),
    W = test_witnessed(),
    {Root, S} = fragmentation@encoding:ingest(
        <<"the the"/utf8>>,
        W,
        fragmentation@store:new()
    ),
    All = fragmentation@walk:collect(Root),
    gleam@list:each(All, fun(F) -> _ = fragmentation@git:write(F, Dir) end),
    Files@1 = case simplifile_erl:read_directory(Dir) of
        {ok, Files} -> Files;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"encoding_test"/utf8>>,
                        function => <<"persist_deduped_encoding_test"/utf8>>,
                        line => 437,
                        value => _assert_fail,
                        start => 14205,
                        'end' => 14258,
                        pattern_start => 14216,
                        pattern_end => 14225})
    end,
    _assert_subject = erlang:length(Files@1),
    _assert_subject@1 = fragmentation@store:size(S),
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"encoding_test"/utf8>>,
                function => <<"persist_deduped_encoding_test"/utf8>>,
                line => 438,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 14268,
                    'end' => 14286
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 14290,
                    'end' => 14303
                    },
                start => 14261,
                'end' => 14303,
                expression_start => 14268})
    end.
