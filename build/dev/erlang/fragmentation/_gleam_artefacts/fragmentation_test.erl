-module(fragmentation_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/fragmentation_test.gleam").
-export([main/0, sha_construction_test/0, hash_returns_sha_test/0, hash_deterministic_test/0, hash_different_input_different_sha_test/0, ref_construction_test/0, author_construction_test/0, committer_construction_test/0, timestamp_construction_test/0, message_construction_test/0, witnessed_construction_test/0, witnessed_serialize_deterministic_test/0, witnessed_fields_in_serialization_test/0, shard_construction_test/0, fragment_construction_test/0, fragment_empty_children_test/0, fragment_multiple_children_test/0, self_ref_shard_test/0, self_ref_fragment_test/0, self_witnessed_test/0, data_shard_test/0, data_fragment_test/0, is_shard_test/0, is_fragment_test/0, children_shard_test/0, hash_fragment_deterministic_test/0, hash_fragment_different_data_test/0, hash_fragment_witnessed_matters_test/0, serialize_roundtrip_hash_test/0, serialize_shard_not_empty_test/0, serialize_fragment_not_empty_test/0, store_new_is_empty_test/0, store_put_and_get_test/0, store_has_test/0, store_size_test/0, store_put_idempotent_test/0, store_get_missing_test/0, store_merge_test/0, store_merge_dedup_test/0, walk_single_shard_test/0, walk_depth_first_test/0, walk_nested_three_levels_test/0, walk_wide_tree_test/0, walk_fold_count_test/0, walk_fold_stop_test/0, walk_fold_collect_data_test/0, walk_depth_shard_test/0, walk_depth_one_level_test/0, walk_depth_two_levels_test/0, walk_depth_asymmetric_test/0, walk_find_test/0, walk_find_not_found_test/0, walk_find_nested_test/0, diff_identical_test/0, diff_different_roots_test/0, diff_added_child_test/0, diff_removed_child_test/0, diff_summary_test/0, diff_summary_empty_test/0, different_witness_different_hash_test/0, parallel_branch_pattern_test/0, trace_chain_test/0, author_committer_split_test/0]).

-file("test/fragmentation_test.gleam", 9).
-spec main() -> nil.
main() ->
    gleeunit:main().

-file("test/fragmentation_test.gleam", 17).
-spec test_witnessed() -> fragmentation:witnessed().
test_witnessed() ->
    fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01T00:00:00Z"/utf8>>),
        fragmentation:message(<<"test"/utf8>>)
    ).

-file("test/fragmentation_test.gleam", 26).
-spec make_shard(binary()) -> fragmentation:fragment().
make_shard(Data) ->
    R = fragmentation:ref(fragmentation:hash(Data), <<"self"/utf8>>),
    fragmentation:shard(R, test_witnessed(), Data).

-file("test/fragmentation_test.gleam", 31).
-spec make_fragment(binary(), list(fragmentation:fragment())) -> fragmentation:fragment().
make_fragment(Label, Children) ->
    R = fragmentation:ref(fragmentation:hash(Label), <<"self"/utf8>>),
    fragmentation:fragment(R, test_witnessed(), Label, Children).

-file("test/fragmentation_test.gleam", 43).
-spec sha_construction_test() -> nil.
sha_construction_test() ->
    S = fragmentation:sha(<<"abc123"/utf8>>),
    _assert_subject = {sha, <<"abc123"/utf8>>},
    case S =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"sha_construction_test"/utf8>>,
                line => 45,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => S,
                    start => 1263,
                    'end' => 1264
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 1268,
                    'end' => 1295
                    },
                start => 1256,
                'end' => 1295,
                expression_start => 1263})
    end.

-file("test/fragmentation_test.gleam", 48).
-spec hash_returns_sha_test() -> nil.
hash_returns_sha_test() ->
    S = fragmentation:hash(<<"test"/utf8>>),
    {sha, Value} = S,
    _assert_subject = string:length(Value),
    _assert_subject@1 = 64,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"hash_returns_sha_test"/utf8>>,
                line => 51,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 1413,
                    'end' => 1433
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 1437,
                    'end' => 1439
                    },
                start => 1406,
                'end' => 1439,
                expression_start => 1413})
    end.

-file("test/fragmentation_test.gleam", 54).
-spec hash_deterministic_test() -> nil.
hash_deterministic_test() ->
    H1 = fragmentation:hash(<<"same"/utf8>>),
    H2 = fragmentation:hash(<<"same"/utf8>>),
    case H1 =:= H2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"hash_deterministic_test"/utf8>>,
                line => 57,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => H1,
                    start => 1563,
                    'end' => 1565
                    },
                right => #{kind => expression,
                    value => H2,
                    start => 1569,
                    'end' => 1571
                    },
                start => 1556,
                'end' => 1571,
                expression_start => 1563})
    end.

-file("test/fragmentation_test.gleam", 60).
-spec hash_different_input_different_sha_test() -> nil.
hash_different_input_different_sha_test() ->
    H1 = fragmentation:hash(<<"hello"/utf8>>),
    H2 = fragmentation:hash(<<"world"/utf8>>),
    case H1 /= H2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"hash_different_input_different_sha_test"/utf8>>,
                line => 63,
                kind => binary_operator,
                operator => '!=',
                left => #{kind => expression,
                    value => H1,
                    start => 1713,
                    'end' => 1715
                    },
                right => #{kind => expression,
                    value => H2,
                    start => 1719,
                    'end' => 1721
                    },
                start => 1706,
                'end' => 1721,
                expression_start => 1713})
    end.

-file("test/fragmentation_test.gleam", 70).
-spec ref_construction_test() -> nil.
ref_construction_test() ->
    S = fragmentation:sha(<<"abc"/utf8>>),
    R = fragmentation:ref(S, <<"parent"/utf8>>),
    _assert_subject = {ref, {sha, <<"abc"/utf8>>}, <<"parent"/utf8>>},
    case R =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"ref_construction_test"/utf8>>,
                line => 73,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => R,
                    start => 2009,
                    'end' => 2010
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 2014,
                    'end' => 2067
                    },
                start => 2002,
                'end' => 2067,
                expression_start => 2009})
    end.

-file("test/fragmentation_test.gleam", 80).
-spec author_construction_test() -> nil.
author_construction_test() ->
    A = fragmentation:author(<<"alex"/utf8>>),
    _assert_subject = {author, <<"alex"/utf8>>},
    case A =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"author_construction_test"/utf8>>,
                line => 82,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => A,
                    start => 2339,
                    'end' => 2340
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 2344,
                    'end' => 2372
                    },
                start => 2332,
                'end' => 2372,
                expression_start => 2339})
    end.

-file("test/fragmentation_test.gleam", 85).
-spec committer_construction_test() -> nil.
committer_construction_test() ->
    C = fragmentation:committer(<<"reed"/utf8>>),
    _assert_subject = {committer, <<"reed"/utf8>>},
    case C =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"committer_construction_test"/utf8>>,
                line => 87,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => C,
                    start => 2466,
                    'end' => 2467
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 2471,
                    'end' => 2502
                    },
                start => 2459,
                'end' => 2502,
                expression_start => 2466})
    end.

-file("test/fragmentation_test.gleam", 90).
-spec timestamp_construction_test() -> nil.
timestamp_construction_test() ->
    T = fragmentation:timestamp(<<"2026-03-01T00:00:00Z"/utf8>>),
    _assert_subject = {timestamp, <<"2026-03-01T00:00:00Z"/utf8>>},
    case T =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"timestamp_construction_test"/utf8>>,
                line => 92,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => T,
                    start => 2612,
                    'end' => 2613
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 2617,
                    'end' => 2664
                    },
                start => 2605,
                'end' => 2664,
                expression_start => 2612})
    end.

-file("test/fragmentation_test.gleam", 95).
-spec message_construction_test() -> nil.
message_construction_test() ->
    M = fragmentation:message(<<"commit msg"/utf8>>),
    _assert_subject = {message, <<"commit msg"/utf8>>},
    case M =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"message_construction_test"/utf8>>,
                line => 97,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => M,
                    start => 2760,
                    'end' => 2761
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 2765,
                    'end' => 2800
                    },
                start => 2753,
                'end' => 2800,
                expression_start => 2760})
    end.

-file("test/fragmentation_test.gleam", 104).
-spec witnessed_construction_test() -> nil.
witnessed_construction_test() ->
    W = fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01T00:00:00Z"/utf8>>),
        fragmentation:message(<<"initial"/utf8>>)
    ),
    _assert_subject = {witnessed,
        {author, <<"alex"/utf8>>},
        {committer, <<"reed"/utf8>>},
        {timestamp, <<"2026-03-01T00:00:00Z"/utf8>>},
        {message, <<"initial"/utf8>>}},
    case W =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"witnessed_construction_test"/utf8>>,
                line => 112,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => W,
                    start => 3234,
                    'end' => 3235
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 3243,
                    'end' => 3443
                    },
                start => 3227,
                'end' => 3443,
                expression_start => 3234})
    end.

-file("test/fragmentation_test.gleam", 121).
-spec witnessed_serialize_deterministic_test() -> nil.
witnessed_serialize_deterministic_test() ->
    W = test_witnessed(),
    S1 = fragmentation:serialize_witnessed(W),
    S2 = fragmentation:serialize_witnessed(W),
    case S1 =:= S2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"witnessed_serialize_deterministic_test"/utf8>>,
                line => 125,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => S1,
                    start => 3629,
                    'end' => 3631
                    },
                right => #{kind => expression,
                    value => S2,
                    start => 3635,
                    'end' => 3637
                    },
                start => 3622,
                'end' => 3637,
                expression_start => 3629})
    end.

-file("test/fragmentation_test.gleam", 128).
-spec witnessed_fields_in_serialization_test() -> nil.
witnessed_fields_in_serialization_test() ->
    W = fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01"/utf8>>),
        fragmentation:message(<<"commit msg"/utf8>>)
    ),
    S = fragmentation:serialize_witnessed(W),
    _assert_subject = <<"author:alex"/utf8>>,
    case gleam_stdlib:contains_string(S, _assert_subject) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"witnessed_fields_in_serialization_test"/utf8>>,
                line => 137,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => S,
                        start => 3971,
                        'end' => 3972
                        }, #{kind => literal,
                        value => _assert_subject,
                        start => 3974,
                        'end' => 3987
                        }],
                start => 3948,
                'end' => 3988,
                expression_start => 3955})
    end,
    _assert_subject@1 = <<"committer:reed"/utf8>>,
    case gleam_stdlib:contains_string(S, _assert_subject@1) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"witnessed_fields_in_serialization_test"/utf8>>,
                line => 138,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => S,
                        start => 4014,
                        'end' => 4015
                        }, #{kind => literal,
                        value => _assert_subject@1,
                        start => 4017,
                        'end' => 4033
                        }],
                start => 3991,
                'end' => 4034,
                expression_start => 3998})
    end,
    _assert_subject@2 = <<"timestamp:2026-03-01"/utf8>>,
    case gleam_stdlib:contains_string(S, _assert_subject@2) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"witnessed_fields_in_serialization_test"/utf8>>,
                line => 139,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => S,
                        start => 4060,
                        'end' => 4061
                        }, #{kind => literal,
                        value => _assert_subject@2,
                        start => 4063,
                        'end' => 4085
                        }],
                start => 4037,
                'end' => 4086,
                expression_start => 4044})
    end,
    _assert_subject@3 = <<"message:commit msg"/utf8>>,
    case gleam_stdlib:contains_string(S, _assert_subject@3) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"witnessed_fields_in_serialization_test"/utf8>>,
                line => 140,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => S,
                        start => 4112,
                        'end' => 4113
                        }, #{kind => literal,
                        value => _assert_subject@3,
                        start => 4115,
                        'end' => 4135
                        }],
                start => 4089,
                'end' => 4136,
                expression_start => 4096})
    end.

-file("test/fragmentation_test.gleam", 147).
-spec shard_construction_test() -> nil.
shard_construction_test() ->
    R = fragmentation:ref(fragmentation:hash(<<"data"/utf8>>), <<"self"/utf8>>),
    W = test_witnessed(),
    S = fragmentation:shard(R, W, <<"hello"/utf8>>),
    _assert_subject = {shard, R, W, <<"hello"/utf8>>},
    case S =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"shard_construction_test"/utf8>>,
                line => 151,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => S,
                    start => 4504,
                    'end' => 4505
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 4509,
                    'end' => 4543
                    },
                start => 4497,
                'end' => 4543,
                expression_start => 4504})
    end.

-file("test/fragmentation_test.gleam", 154).
-spec fragment_construction_test() -> nil.
fragment_construction_test() ->
    Leaf = make_shard(<<"leaf-data"/utf8>>),
    R = fragmentation:ref(fragmentation:hash(<<"root"/utf8>>), <<"self"/utf8>>),
    W = test_witnessed(),
    F = fragmentation:fragment(R, W, <<"root-data"/utf8>>, [Leaf]),
    _assert_subject = {fragment, R, W, <<"root-data"/utf8>>, [Leaf]},
    case F =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"fragment_construction_test"/utf8>>,
                line => 159,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => F,
                    start => 4782,
                    'end' => 4783
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 4787,
                    'end' => 4836
                    },
                start => 4775,
                'end' => 4836,
                expression_start => 4782})
    end.

-file("test/fragmentation_test.gleam", 162).
-spec fragment_empty_children_test() -> nil.
fragment_empty_children_test() ->
    F = make_fragment(<<"empty"/utf8>>, []),
    _assert_subject = fragmentation:children(F),
    _assert_subject@1 = [],
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"fragment_empty_children_test"/utf8>>,
                line => 164,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 4926,
                    'end' => 4951
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 4955,
                    'end' => 4957
                    },
                start => 4919,
                'end' => 4957,
                expression_start => 4926})
    end.

-file("test/fragmentation_test.gleam", 167).
-spec fragment_multiple_children_test() -> nil.
fragment_multiple_children_test() ->
    A = make_shard(<<"alpha"/utf8>>),
    B = make_shard(<<"beta"/utf8>>),
    F = make_fragment(<<"parent"/utf8>>, [A, B]),
    _assert_subject = fragmentation:children(F),
    _assert_subject@1 = [A, B],
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"fragment_multiple_children_test"/utf8>>,
                line => 171,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5114,
                    'end' => 5139
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 5143,
                    'end' => 5149
                    },
                start => 5107,
                'end' => 5149,
                expression_start => 5114})
    end.

-file("test/fragmentation_test.gleam", 178).
-spec self_ref_shard_test() -> nil.
self_ref_shard_test() ->
    S = make_shard(<<"data"/utf8>>),
    R = fragmentation:self_ref(S),
    {ref, Sha, _} = R,
    _assert_subject = fragmentation:hash(<<"data"/utf8>>),
    case Sha =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"self_ref_shard_test"/utf8>>,
                line => 182,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Sha,
                    start => 5464,
                    'end' => 5467
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 5471,
                    'end' => 5497
                    },
                start => 5457,
                'end' => 5497,
                expression_start => 5464})
    end.

-file("test/fragmentation_test.gleam", 185).
-spec self_ref_fragment_test() -> nil.
self_ref_fragment_test() ->
    F = make_fragment(<<"node"/utf8>>, []),
    R = fragmentation:self_ref(F),
    {ref, Sha, _} = R,
    _assert_subject = fragmentation:hash(<<"node"/utf8>>),
    case Sha =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"self_ref_fragment_test"/utf8>>,
                line => 189,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Sha,
                    start => 5652,
                    'end' => 5655
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 5659,
                    'end' => 5685
                    },
                start => 5645,
                'end' => 5685,
                expression_start => 5652})
    end.

-file("test/fragmentation_test.gleam", 192).
-spec self_witnessed_test() -> nil.
self_witnessed_test() ->
    S = make_shard(<<"x"/utf8>>),
    _assert_subject = fragmentation:self_witnessed(S),
    _assert_subject@1 = test_witnessed(),
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"self_witnessed_test"/utf8>>,
                line => 194,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5755,
                    'end' => 5786
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 5790,
                    'end' => 5806
                    },
                start => 5748,
                'end' => 5806,
                expression_start => 5755})
    end.

-file("test/fragmentation_test.gleam", 197).
-spec data_shard_test() -> nil.
data_shard_test() ->
    S = make_shard(<<"payload"/utf8>>),
    _assert_subject = fragmentation:data(S),
    _assert_subject@1 = <<"payload"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"data_shard_test"/utf8>>,
                line => 199,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5878,
                    'end' => 5899
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 5903,
                    'end' => 5912
                    },
                start => 5871,
                'end' => 5912,
                expression_start => 5878})
    end.

-file("test/fragmentation_test.gleam", 202).
-spec data_fragment_test() -> nil.
data_fragment_test() ->
    F = make_fragment(<<"payload"/utf8>>, []),
    _assert_subject = fragmentation:data(F),
    _assert_subject@1 = <<"payload"/utf8>>,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"data_fragment_test"/utf8>>,
                line => 204,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 5994,
                    'end' => 6015
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 6019,
                    'end' => 6028
                    },
                start => 5987,
                'end' => 6028,
                expression_start => 5994})
    end.

-file("test/fragmentation_test.gleam", 207).
-spec is_shard_test() -> nil.
is_shard_test() ->
    _assert_subject = fragmentation:is_shard(make_shard(<<"x"/utf8>>)),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"is_shard_test"/utf8>>,
                line => 208,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 6066,
                    'end' => 6105
                    },
                right => #{kind => literal,
                    value => true,
                    start => 6109,
                    'end' => 6113
                    },
                start => 6059,
                'end' => 6113,
                expression_start => 6066})
    end,
    _assert_subject@1 = fragmentation:is_shard(make_fragment(<<"x"/utf8>>, [])),
    case _assert_subject@1 =:= false of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"is_shard_test"/utf8>>,
                line => 209,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@1,
                    start => 6123,
                    'end' => 6169
                    },
                right => #{kind => literal,
                    value => false,
                    start => 6173,
                    'end' => 6178
                    },
                start => 6116,
                'end' => 6178,
                expression_start => 6123})
    end.

-file("test/fragmentation_test.gleam", 212).
-spec is_fragment_test() -> nil.
is_fragment_test() ->
    _assert_subject = fragmentation:is_fragment(make_fragment(<<"x"/utf8>>, [])),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"is_fragment_test"/utf8>>,
                line => 213,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 6219,
                    'end' => 6268
                    },
                right => #{kind => literal,
                    value => true,
                    start => 6272,
                    'end' => 6276
                    },
                start => 6212,
                'end' => 6276,
                expression_start => 6219})
    end,
    _assert_subject@1 = fragmentation:is_fragment(make_shard(<<"x"/utf8>>)),
    case _assert_subject@1 =:= false of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"is_fragment_test"/utf8>>,
                line => 214,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@1,
                    start => 6286,
                    'end' => 6328
                    },
                right => #{kind => literal,
                    value => false,
                    start => 6332,
                    'end' => 6337
                    },
                start => 6279,
                'end' => 6337,
                expression_start => 6286})
    end.

-file("test/fragmentation_test.gleam", 217).
-spec children_shard_test() -> nil.
children_shard_test() ->
    _assert_subject = fragmentation:children(make_shard(<<"x"/utf8>>)),
    _assert_subject@1 = [],
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"children_shard_test"/utf8>>,
                line => 218,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 6381,
                    'end' => 6420
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 6424,
                    'end' => 6426
                    },
                start => 6374,
                'end' => 6426,
                expression_start => 6381})
    end.

-file("test/fragmentation_test.gleam", 225).
-spec hash_fragment_deterministic_test() -> nil.
hash_fragment_deterministic_test() ->
    S = make_shard(<<"hello"/utf8>>),
    H1 = fragmentation:hash_fragment(S),
    H2 = fragmentation:hash_fragment(S),
    case H1 =:= H2 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"hash_fragment_deterministic_test"/utf8>>,
                line => 229,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => H1,
                    start => 6778,
                    'end' => 6780
                    },
                right => #{kind => expression,
                    value => H2,
                    start => 6784,
                    'end' => 6786
                    },
                start => 6771,
                'end' => 6786,
                expression_start => 6778})
    end.

-file("test/fragmentation_test.gleam", 232).
-spec hash_fragment_different_data_test() -> nil.
hash_fragment_different_data_test() ->
    S1 = make_shard(<<"hello"/utf8>>),
    S2 = make_shard(<<"world"/utf8>>),
    _assert_subject = fragmentation:hash_fragment(S1),
    _assert_subject@1 = fragmentation:hash_fragment(S2),
    case _assert_subject /= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"hash_fragment_different_data_test"/utf8>>,
                line => 235,
                kind => binary_operator,
                operator => '!=',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 6906,
                    'end' => 6937
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 6941,
                    'end' => 6972
                    },
                start => 6899,
                'end' => 6972,
                expression_start => 6906})
    end.

-file("test/fragmentation_test.gleam", 238).
-spec hash_fragment_witnessed_matters_test() -> nil.
hash_fragment_witnessed_matters_test() ->
    R = fragmentation:ref(fragmentation:hash(<<"x"/utf8>>), <<"self"/utf8>>),
    W1 = fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01"/utf8>>),
        fragmentation:message(<<"first"/utf8>>)
    ),
    W2 = fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01"/utf8>>),
        fragmentation:message(<<"second"/utf8>>)
    ),
    S1 = fragmentation:shard(R, W1, <<"same-data"/utf8>>),
    S2 = fragmentation:shard(R, W2, <<"same-data"/utf8>>),
    _assert_subject = fragmentation:hash_fragment(S1),
    _assert_subject@1 = fragmentation:hash_fragment(S2),
    case _assert_subject /= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"hash_fragment_witnessed_matters_test"/utf8>>,
                line => 257,
                kind => binary_operator,
                operator => '!=',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 7685,
                    'end' => 7716
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 7720,
                    'end' => 7751
                    },
                start => 7678,
                'end' => 7751,
                expression_start => 7685})
    end.

-file("test/fragmentation_test.gleam", 260).
-spec serialize_roundtrip_hash_test() -> nil.
serialize_roundtrip_hash_test() ->
    S = make_shard(<<"test"/utf8>>),
    Hash_direct = fragmentation:hash_fragment(S),
    {sha, Hash_via_serial} = fragmentation:hash(fragmentation:serialize(S)),
    case Hash_direct =:= Hash_via_serial of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"serialize_roundtrip_hash_test"/utf8>>,
                line => 265,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Hash_direct,
                    start => 7979,
                    'end' => 7990
                    },
                right => #{kind => expression,
                    value => Hash_via_serial,
                    start => 7994,
                    'end' => 8009
                    },
                start => 7972,
                'end' => 8009,
                expression_start => 7979})
    end.

-file("test/fragmentation_test.gleam", 268).
-spec serialize_shard_not_empty_test() -> nil.
serialize_shard_not_empty_test() ->
    S = make_shard(<<"data"/utf8>>),
    _assert_subject = fragmentation:serialize(S),
    _assert_subject@1 = <<""/utf8>>,
    case _assert_subject /= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"serialize_shard_not_empty_test"/utf8>>,
                line => 270,
                kind => binary_operator,
                operator => '!=',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8093,
                    'end' => 8119
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 8123,
                    'end' => 8125
                    },
                start => 8086,
                'end' => 8125,
                expression_start => 8093})
    end.

-file("test/fragmentation_test.gleam", 273).
-spec serialize_fragment_not_empty_test() -> nil.
serialize_fragment_not_empty_test() ->
    F = make_fragment(<<"root"/utf8>>, [make_shard(<<"leaf"/utf8>>)]),
    _assert_subject = fragmentation:serialize(F),
    _assert_subject@1 = <<""/utf8>>,
    case _assert_subject /= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"serialize_fragment_not_empty_test"/utf8>>,
                line => 275,
                kind => binary_operator,
                operator => '!=',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8237,
                    'end' => 8263
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 8267,
                    'end' => 8269
                    },
                start => 8230,
                'end' => 8269,
                expression_start => 8237})
    end.

-file("test/fragmentation_test.gleam", 282).
-spec store_new_is_empty_test() -> nil.
store_new_is_empty_test() ->
    S = fragmentation@store:new(),
    _assert_subject = fragmentation@store:size(S),
    _assert_subject@1 = 0,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_new_is_empty_test"/utf8>>,
                line => 284,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8507,
                    'end' => 8520
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 8524,
                    'end' => 8525
                    },
                start => 8500,
                'end' => 8525,
                expression_start => 8507})
    end.

-file("test/fragmentation_test.gleam", 287).
-spec store_put_and_get_test() -> nil.
store_put_and_get_test() ->
    Frag = make_shard(<<"hello"/utf8>>),
    S = fragmentation@store:put(fragmentation@store:new(), Frag),
    {ref, Sha, _} = fragmentation:self_ref(Frag),
    _assert_subject = fragmentation@store:get(S, Sha),
    _assert_subject@1 = {ok, Frag},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_put_and_get_test"/utf8>>,
                line => 291,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8707,
                    'end' => 8724
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 8728,
                    'end' => 8736
                    },
                start => 8700,
                'end' => 8736,
                expression_start => 8707})
    end.

-file("test/fragmentation_test.gleam", 294).
-spec store_has_test() -> nil.
store_has_test() ->
    Frag = make_shard(<<"exists"/utf8>>),
    S = fragmentation@store:put(fragmentation@store:new(), Frag),
    {ref, Sha, _} = fragmentation:self_ref(Frag),
    _assert_subject = fragmentation@store:has(S, Sha),
    case _assert_subject =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_has_test"/utf8>>,
                line => 298,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 8911,
                    'end' => 8928
                    },
                right => #{kind => literal,
                    value => true,
                    start => 8932,
                    'end' => 8936
                    },
                start => 8904,
                'end' => 8936,
                expression_start => 8911})
    end,
    _assert_subject@1 = fragmentation@store:has(
        S,
        fragmentation:sha(<<"nonexistent"/utf8>>)
    ),
    case _assert_subject@1 =:= false of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_has_test"/utf8>>,
                line => 299,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@1,
                    start => 8946,
                    'end' => 8992
                    },
                right => #{kind => literal,
                    value => false,
                    start => 8996,
                    'end' => 9001
                    },
                start => 8939,
                'end' => 9001,
                expression_start => 8946})
    end.

-file("test/fragmentation_test.gleam", 302).
-spec store_size_test() -> nil.
store_size_test() ->
    S = fragmentation@store:new(),
    _assert_subject = fragmentation@store:size(S),
    _assert_subject@1 = 0,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_size_test"/utf8>>,
                line => 304,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 9063,
                    'end' => 9076
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 9080,
                    'end' => 9081
                    },
                start => 9056,
                'end' => 9081,
                expression_start => 9063})
    end,
    S@1 = fragmentation@store:put(S, make_shard(<<"a"/utf8>>)),
    _assert_subject@2 = fragmentation@store:size(S@1),
    _assert_subject@3 = 1,
    case _assert_subject@2 =:= _assert_subject@3 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_size_test"/utf8>>,
                line => 306,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@2,
                    start => 9131,
                    'end' => 9144
                    },
                right => #{kind => literal,
                    value => _assert_subject@3,
                    start => 9148,
                    'end' => 9149
                    },
                start => 9124,
                'end' => 9149,
                expression_start => 9131})
    end,
    S@2 = fragmentation@store:put(S@1, make_shard(<<"b"/utf8>>)),
    _assert_subject@4 = fragmentation@store:size(S@2),
    _assert_subject@5 = 2,
    case _assert_subject@4 =:= _assert_subject@5 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_size_test"/utf8>>,
                line => 308,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@4,
                    start => 9199,
                    'end' => 9212
                    },
                right => #{kind => literal,
                    value => _assert_subject@5,
                    start => 9216,
                    'end' => 9217
                    },
                start => 9192,
                'end' => 9217,
                expression_start => 9199})
    end.

-file("test/fragmentation_test.gleam", 311).
-spec store_put_idempotent_test() -> nil.
store_put_idempotent_test() ->
    Frag = make_shard(<<"same"/utf8>>),
    S = fragmentation@store:put(fragmentation@store:new(), Frag),
    S@1 = fragmentation@store:put(S, Frag),
    _assert_subject = fragmentation@store:size(S@1),
    _assert_subject@1 = 1,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_put_idempotent_test"/utf8>>,
                line => 315,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 9367,
                    'end' => 9380
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 9384,
                    'end' => 9385
                    },
                start => 9360,
                'end' => 9385,
                expression_start => 9367})
    end.

-file("test/fragmentation_test.gleam", 318).
-spec store_get_missing_test() -> nil.
store_get_missing_test() ->
    S = fragmentation@store:new(),
    _assert_subject = fragmentation@store:get(
        S,
        fragmentation:sha(<<"nope"/utf8>>)
    ),
    _assert_subject@1 = {error, nil},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_get_missing_test"/utf8>>,
                line => 320,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 9454,
                    'end' => 9493
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 9497,
                    'end' => 9507
                    },
                start => 9447,
                'end' => 9507,
                expression_start => 9454})
    end.

-file("test/fragmentation_test.gleam", 323).
-spec store_merge_test() -> nil.
store_merge_test() ->
    A = fragmentation@store:put(
        fragmentation@store:new(),
        make_shard(<<"alpha"/utf8>>)
    ),
    B = fragmentation@store:put(
        fragmentation@store:new(),
        make_shard(<<"beta"/utf8>>)
    ),
    Merged = fragmentation@store:merge(A, B),
    _assert_subject = fragmentation@store:size(Merged),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_merge_test"/utf8>>,
                line => 327,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 9688,
                    'end' => 9706
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 9710,
                    'end' => 9711
                    },
                start => 9681,
                'end' => 9711,
                expression_start => 9688})
    end.

-file("test/fragmentation_test.gleam", 330).
-spec store_merge_dedup_test() -> nil.
store_merge_dedup_test() ->
    Frag = make_shard(<<"shared"/utf8>>),
    A = fragmentation@store:put(fragmentation@store:new(), Frag),
    B = fragmentation@store:put(fragmentation@store:new(), Frag),
    Merged = fragmentation@store:merge(A, B),
    _assert_subject = fragmentation@store:size(Merged),
    _assert_subject@1 = 1,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"store_merge_dedup_test"/utf8>>,
                line => 335,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 9903,
                    'end' => 9921
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 9925,
                    'end' => 9926
                    },
                start => 9896,
                'end' => 9926,
                expression_start => 9903})
    end.

-file("test/fragmentation_test.gleam", 342).
-spec walk_single_shard_test() -> nil.
walk_single_shard_test() ->
    S = make_shard(<<"leaf"/utf8>>),
    Result = fragmentation@walk:collect(S),
    _assert_subject = [S],
    case Result =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_single_shard_test"/utf8>>,
                line => 345,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Result,
                    start => 10200,
                    'end' => 10206
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 10210,
                    'end' => 10213
                    },
                start => 10193,
                'end' => 10213,
                expression_start => 10200})
    end.

-file("test/fragmentation_test.gleam", 348).
-spec walk_depth_first_test() -> nil.
walk_depth_first_test() ->
    Leaf = make_shard(<<"leaf"/utf8>>),
    Parent = make_fragment(<<"parent"/utf8>>, [Leaf]),
    Collected = fragmentation@walk:collect(Parent),
    _assert_subject = erlang:length(Collected),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_depth_first_test"/utf8>>,
                line => 352,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 10377,
                    'end' => 10399
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 10403,
                    'end' => 10404
                    },
                start => 10370,
                'end' => 10404,
                expression_start => 10377})
    end,
    First@1 = case gleam@list:first(Collected) of
        {ok, First} -> First;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"fragmentation_test"/utf8>>,
                        function => <<"walk_depth_first_test"/utf8>>,
                        line => 353,
                        value => _assert_fail,
                        start => 10407,
                        'end' => 10451,
                        pattern_start => 10418,
                        pattern_end => 10427})
    end,
    case First@1 =:= Parent of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_depth_first_test"/utf8>>,
                line => 354,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => First@1,
                    start => 10461,
                    'end' => 10466
                    },
                right => #{kind => expression,
                    value => Parent,
                    start => 10470,
                    'end' => 10476
                    },
                start => 10454,
                'end' => 10476,
                expression_start => 10461})
    end.

-file("test/fragmentation_test.gleam", 357).
-spec walk_nested_three_levels_test() -> nil.
walk_nested_three_levels_test() ->
    Leaf = make_shard(<<"leaf"/utf8>>),
    Mid = make_fragment(<<"mid"/utf8>>, [Leaf]),
    Root = make_fragment(<<"root"/utf8>>, [Mid]),
    Collected = fragmentation@walk:collect(Root),
    _assert_subject = erlang:length(Collected),
    _assert_subject@1 = 3,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_nested_three_levels_test"/utf8>>,
                line => 362,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 10682,
                    'end' => 10704
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 10708,
                    'end' => 10709
                    },
                start => 10675,
                'end' => 10709,
                expression_start => 10682})
    end.

-file("test/fragmentation_test.gleam", 365).
-spec walk_wide_tree_test() -> nil.
walk_wide_tree_test() ->
    A = make_shard(<<"a"/utf8>>),
    B = make_shard(<<"b"/utf8>>),
    C = make_shard(<<"c"/utf8>>),
    Root = make_fragment(<<"root"/utf8>>, [A, B, C]),
    Collected = fragmentation@walk:collect(Root),
    _assert_subject = erlang:length(Collected),
    _assert_subject@1 = 4,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_wide_tree_test"/utf8>>,
                line => 371,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 10914,
                    'end' => 10936
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 10940,
                    'end' => 10941
                    },
                start => 10907,
                'end' => 10941,
                expression_start => 10914})
    end.

-file("test/fragmentation_test.gleam", 374).
-spec walk_fold_count_test() -> nil.
walk_fold_count_test() ->
    Root = make_fragment(
        <<"root"/utf8>>,
        [make_shard(<<"a"/utf8>>), make_shard(<<"b"/utf8>>)]
    ),
    Count = fragmentation@walk:fold(
        Root,
        0,
        fun(Acc, _) -> {continue, Acc + 1} end
    ),
    _assert_subject = 3,
    case Count =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_fold_count_test"/utf8>>,
                line => 377,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Count,
                    start => 11133,
                    'end' => 11138
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 11142,
                    'end' => 11143
                    },
                start => 11126,
                'end' => 11143,
                expression_start => 11133})
    end.

-file("test/fragmentation_test.gleam", 380).
-spec walk_fold_stop_test() -> nil.
walk_fold_stop_test() ->
    Root = make_fragment(
        <<"root"/utf8>>,
        [make_shard(<<"a"/utf8>>), make_shard(<<"b"/utf8>>)]
    ),
    Count = fragmentation@walk:fold(Root, 0, fun(Acc, _) -> {stop, Acc + 1} end),
    _assert_subject = 1,
    case Count =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_fold_stop_test"/utf8>>,
                line => 383,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Count,
                    start => 11330,
                    'end' => 11335
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 11339,
                    'end' => 11340
                    },
                start => 11323,
                'end' => 11340,
                expression_start => 11330})
    end.

-file("test/fragmentation_test.gleam", 386).
-spec walk_fold_collect_data_test() -> nil.
walk_fold_collect_data_test() ->
    Root = make_fragment(
        <<"root"/utf8>>,
        [make_shard(<<"a"/utf8>>), make_shard(<<"b"/utf8>>)]
    ),
    Data = fragmentation@walk:fold(
        Root,
        [],
        fun(Acc, Frag) -> {continue, [fragmentation:data(Frag) | Acc]} end
    ),
    _assert_subject = erlang:length(Data),
    _assert_subject@1 = 3,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_fold_collect_data_test"/utf8>>,
                line => 392,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 11578,
                    'end' => 11595
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 11599,
                    'end' => 11600
                    },
                start => 11571,
                'end' => 11600,
                expression_start => 11578})
    end,
    _assert_subject@2 = <<"a"/utf8>>,
    case gleam@list:contains(Data, _assert_subject@2) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_fold_collect_data_test"/utf8>>,
                line => 393,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => Data,
                        start => 11624,
                        'end' => 11628
                        }, #{kind => literal,
                        value => _assert_subject@2,
                        start => 11630,
                        'end' => 11633
                        }],
                start => 11603,
                'end' => 11634,
                expression_start => 11610})
    end,
    _assert_subject@3 = <<"b"/utf8>>,
    case gleam@list:contains(Data, _assert_subject@3) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_fold_collect_data_test"/utf8>>,
                line => 394,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => Data,
                        start => 11658,
                        'end' => 11662
                        }, #{kind => literal,
                        value => _assert_subject@3,
                        start => 11664,
                        'end' => 11667
                        }],
                start => 11637,
                'end' => 11668,
                expression_start => 11644})
    end,
    _assert_subject@4 = <<"root"/utf8>>,
    case gleam@list:contains(Data, _assert_subject@4) of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_fold_collect_data_test"/utf8>>,
                line => 395,
                kind => function_call,
                arguments => [#{kind => expression,
                        value => Data,
                        start => 11692,
                        'end' => 11696
                        }, #{kind => literal,
                        value => _assert_subject@4,
                        start => 11698,
                        'end' => 11704
                        }],
                start => 11671,
                'end' => 11705,
                expression_start => 11678})
    end.

-file("test/fragmentation_test.gleam", 398).
-spec walk_depth_shard_test() -> nil.
walk_depth_shard_test() ->
    _assert_subject = fragmentation@walk:depth(make_shard(<<"x"/utf8>>)),
    _assert_subject@1 = 0,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_depth_shard_test"/utf8>>,
                line => 399,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 11751,
                    'end' => 11778
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 11782,
                    'end' => 11783
                    },
                start => 11744,
                'end' => 11783,
                expression_start => 11751})
    end.

-file("test/fragmentation_test.gleam", 402).
-spec walk_depth_one_level_test() -> nil.
walk_depth_one_level_test() ->
    Parent = make_fragment(<<"parent"/utf8>>, [make_shard(<<"leaf"/utf8>>)]),
    _assert_subject = fragmentation@walk:depth(Parent),
    _assert_subject@1 = 1,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_depth_one_level_test"/utf8>>,
                line => 404,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 11894,
                    'end' => 11912
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 11916,
                    'end' => 11917
                    },
                start => 11887,
                'end' => 11917,
                expression_start => 11894})
    end.

-file("test/fragmentation_test.gleam", 407).
-spec walk_depth_two_levels_test() -> nil.
walk_depth_two_levels_test() ->
    Leaf = make_shard(<<"leaf"/utf8>>),
    Mid = make_fragment(<<"mid"/utf8>>, [Leaf]),
    Root = make_fragment(<<"root"/utf8>>, [Mid]),
    _assert_subject = fragmentation@walk:depth(Root),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_depth_two_levels_test"/utf8>>,
                line => 411,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 12083,
                    'end' => 12099
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 12103,
                    'end' => 12104
                    },
                start => 12076,
                'end' => 12104,
                expression_start => 12083})
    end.

-file("test/fragmentation_test.gleam", 414).
-spec walk_depth_asymmetric_test() -> nil.
walk_depth_asymmetric_test() ->
    Deep = make_fragment(<<"deep"/utf8>>, [make_shard(<<"leaf"/utf8>>)]),
    Shallow = make_shard(<<"shallow"/utf8>>),
    Root = make_fragment(<<"root"/utf8>>, [Deep, Shallow]),
    _assert_subject = fragmentation@walk:depth(Root),
    _assert_subject@1 = 2,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_depth_asymmetric_test"/utf8>>,
                line => 419,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 12340,
                    'end' => 12356
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 12360,
                    'end' => 12361
                    },
                start => 12333,
                'end' => 12361,
                expression_start => 12340})
    end.

-file("test/fragmentation_test.gleam", 422).
-spec walk_find_test() -> nil.
walk_find_test() ->
    Target = make_shard(<<"needle"/utf8>>),
    Other = make_shard(<<"hay"/utf8>>),
    Root = make_fragment(<<"root"/utf8>>, [Other, Target]),
    Result = fragmentation@walk:find(
        Root,
        fun(F) -> fragmentation:data(F) =:= <<"needle"/utf8>> end
    ),
    _assert_subject = {ok, Target},
    case Result =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_find_test"/utf8>>,
                line => 427,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Result,
                    start => 12596,
                    'end' => 12602
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 12606,
                    'end' => 12616
                    },
                start => 12589,
                'end' => 12616,
                expression_start => 12596})
    end.

-file("test/fragmentation_test.gleam", 430).
-spec walk_find_not_found_test() -> nil.
walk_find_not_found_test() ->
    S = make_shard(<<"x"/utf8>>),
    Result = fragmentation@walk:find(
        S,
        fun(F) -> fragmentation:data(F) =:= <<"missing"/utf8>> end
    ),
    _assert_subject = {error, nil},
    case Result =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_find_not_found_test"/utf8>>,
                line => 433,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Result,
                    start => 12765,
                    'end' => 12771
                    },
                right => #{kind => literal,
                    value => _assert_subject,
                    start => 12775,
                    'end' => 12785
                    },
                start => 12758,
                'end' => 12785,
                expression_start => 12765})
    end.

-file("test/fragmentation_test.gleam", 436).
-spec walk_find_nested_test() -> nil.
walk_find_nested_test() ->
    Target = make_shard(<<"deep-needle"/utf8>>),
    Mid = make_fragment(<<"mid"/utf8>>, [Target]),
    Root = make_fragment(<<"root"/utf8>>, [make_shard(<<"hay"/utf8>>), Mid]),
    Result = fragmentation@walk:find(
        Root,
        fun(F) -> fragmentation:data(F) =:= <<"deep-needle"/utf8>> end
    ),
    _assert_subject = {ok, Target},
    case Result =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"walk_find_nested_test"/utf8>>,
                line => 441,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Result,
                    start => 13057,
                    'end' => 13063
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 13067,
                    'end' => 13077
                    },
                start => 13050,
                'end' => 13077,
                expression_start => 13057})
    end.

-file("test/fragmentation_test.gleam", 448).
-spec diff_identical_test() -> nil.
diff_identical_test() ->
    S = make_shard(<<"same"/utf8>>),
    Changes = fragmentation@diff:diff(S, S),
    _assert_subject = [{unchanged, S}],
    case Changes =:= _assert_subject of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"diff_identical_test"/utf8>>,
                line => 451,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Changes,
                    start => 13349,
                    'end' => 13356
                    },
                right => #{kind => expression,
                    value => _assert_subject,
                    start => 13360,
                    'end' => 13379
                    },
                start => 13342,
                'end' => 13379,
                expression_start => 13349})
    end.

-file("test/fragmentation_test.gleam", 454).
-spec diff_different_roots_test() -> nil.
diff_different_roots_test() ->
    Old = make_shard(<<"old"/utf8>>),
    New = make_shard(<<"new"/utf8>>),
    Changes = fragmentation@diff:diff(Old, New),
    Has_modified = gleam@list:any(Changes, fun(C) -> case C of
                {modified, _, _} ->
                    true;

                _ ->
                    false
            end end),
    case Has_modified =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"diff_different_roots_test"/utf8>>,
                line => 465,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Has_modified,
                    start => 13661,
                    'end' => 13673
                    },
                right => #{kind => literal,
                    value => true,
                    start => 13677,
                    'end' => 13681
                    },
                start => 13654,
                'end' => 13681,
                expression_start => 13661})
    end.

-file("test/fragmentation_test.gleam", 468).
-spec diff_added_child_test() -> nil.
diff_added_child_test() ->
    Child = make_shard(<<"child"/utf8>>),
    Old = make_fragment(<<"root"/utf8>>, []),
    New = make_fragment(<<"root"/utf8>>, [Child]),
    Changes = fragmentation@diff:diff(Old, New),
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
                module => <<"fragmentation_test"/utf8>>,
                function => <<"diff_added_child_test"/utf8>>,
                line => 480,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Has_added,
                    start => 14005,
                    'end' => 14014
                    },
                right => #{kind => literal,
                    value => true,
                    start => 14018,
                    'end' => 14022
                    },
                start => 13998,
                'end' => 14022,
                expression_start => 14005})
    end.

-file("test/fragmentation_test.gleam", 483).
-spec diff_removed_child_test() -> nil.
diff_removed_child_test() ->
    Child = make_shard(<<"child"/utf8>>),
    Old = make_fragment(<<"root"/utf8>>, [Child]),
    New = make_fragment(<<"root"/utf8>>, []),
    Changes = fragmentation@diff:diff(Old, New),
    Has_removed = gleam@list:any(Changes, fun(C) -> case C of
                {removed, _} ->
                    true;

                _ ->
                    false
            end end),
    case Has_removed =:= true of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"diff_removed_child_test"/utf8>>,
                line => 495,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => Has_removed,
                    start => 14352,
                    'end' => 14363
                    },
                right => #{kind => literal,
                    value => true,
                    start => 14367,
                    'end' => 14371
                    },
                start => 14345,
                'end' => 14371,
                expression_start => 14352})
    end.

-file("test/fragmentation_test.gleam", 498).
-spec diff_summary_test() -> nil.
diff_summary_test() ->
    Changes = [{added, make_shard(<<"x"/utf8>>)},
        {removed, make_shard(<<"y"/utf8>>)},
        {modified, make_shard(<<"old"/utf8>>), make_shard(<<"new"/utf8>>)},
        {unchanged, make_shard(<<"z"/utf8>>)},
        {unchanged, make_shard(<<"w"/utf8>>)}],
    _assert_subject = fragmentation@diff:summary(Changes),
    _assert_subject@1 = {1, 1, 1, 2},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"diff_summary_test"/utf8>>,
                line => 506,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 14634,
                    'end' => 14655
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 14659,
                    'end' => 14672
                    },
                start => 14627,
                'end' => 14672,
                expression_start => 14634})
    end.

-file("test/fragmentation_test.gleam", 509).
-spec diff_summary_empty_test() -> nil.
diff_summary_empty_test() ->
    _assert_subject = fragmentation@diff:summary([]),
    _assert_subject@1 = {0, 0, 0, 0},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"diff_summary_empty_test"/utf8>>,
                line => 510,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 14720,
                    'end' => 14736
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 14740,
                    'end' => 14753
                    },
                start => 14713,
                'end' => 14753,
                expression_start => 14720})
    end.

-file("test/fragmentation_test.gleam", 517).
-spec different_witness_different_hash_test() -> nil.
different_witness_different_hash_test() ->
    R = fragmentation:ref(fragmentation:hash(<<"x"/utf8>>), <<"self"/utf8>>),
    W_alex = fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"alex"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01"/utf8>>),
        fragmentation:message(<<"observed"/utf8>>)
    ),
    W_reed = fragmentation:witnessed(
        fragmentation:author(<<"reed"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01"/utf8>>),
        fragmentation:message(<<"traced"/utf8>>)
    ),
    S_alex = fragmentation:shard(R, W_alex, <<"same-data"/utf8>>),
    S_reed = fragmentation:shard(R, W_reed, <<"same-data"/utf8>>),
    _assert_subject = fragmentation:hash_fragment(S_alex),
    _assert_subject@1 = fragmentation:hash_fragment(S_reed),
    case _assert_subject /= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"different_witness_different_hash_test"/utf8>>,
                line => 536,
                kind => binary_operator,
                operator => '!=',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 15684,
                    'end' => 15719
                    },
                right => #{kind => expression,
                    value => _assert_subject@1,
                    start => 15727,
                    'end' => 15762
                    },
                start => 15677,
                'end' => 15762,
                expression_start => 15684})
    end.

-file("test/fragmentation_test.gleam", 540).
-spec parallel_branch_pattern_test() -> nil.
parallel_branch_pattern_test() ->
    Decision = make_shard(<<"decision:allow"/utf8>>),
    Bias_root = make_fragment(<<"bias"/utf8>>, [Decision]),
    Trace = make_fragment(<<"trace"/utf8>>, [Bias_root]),
    Collected = fragmentation@walk:collect(Trace),
    _assert_subject = erlang:length(Collected),
    _assert_subject@1 = 3,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"parallel_branch_pattern_test"/utf8>>,
                line => 550,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 16156,
                    'end' => 16178
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 16182,
                    'end' => 16183
                    },
                start => 16149,
                'end' => 16183,
                expression_start => 16156})
    end.

-file("test/fragmentation_test.gleam", 553).
-spec trace_chain_test() -> nil.
trace_chain_test() ->
    Bias = make_shard(<<"bias:v1"/utf8>>),
    T1 = make_fragment(<<"step:observe"/utf8>>, [Bias]),
    T2 = make_fragment(<<"step:decide"/utf8>>, [T1]),
    T3 = make_fragment(<<"step:act"/utf8>>, [T2]),
    _assert_subject = fragmentation@walk:depth(T3),
    _assert_subject@1 = 3,
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"trace_chain_test"/utf8>>,
                line => 560,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 16464,
                    'end' => 16478
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 16482,
                    'end' => 16483
                    },
                start => 16457,
                'end' => 16483,
                expression_start => 16464})
    end,
    Collected = fragmentation@walk:collect(T3),
    _assert_subject@2 = erlang:length(Collected),
    _assert_subject@3 = 4,
    case _assert_subject@2 =:= _assert_subject@3 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"trace_chain_test"/utf8>>,
                line => 562,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@2,
                    start => 16528,
                    'end' => 16550
                    },
                right => #{kind => literal,
                    value => _assert_subject@3,
                    start => 16554,
                    'end' => 16555
                    },
                start => 16521,
                'end' => 16555,
                expression_start => 16528})
    end.

-file("test/fragmentation_test.gleam", 565).
-spec author_committer_split_test() -> nil.
author_committer_split_test() ->
    R = fragmentation:ref(
        fragmentation:hash(<<"decision"/utf8>>),
        <<"self"/utf8>>
    ),
    W = fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01T19:30:00Z"/utf8>>),
        fragmentation:message(<<"bias execution trace"/utf8>>)
    ),
    Traced = fragmentation:shard(R, W, <<"decision:allow"/utf8>>),
    Witness = fragmentation:self_witnessed(Traced),
    _assert_subject = erlang:element(2, Witness),
    _assert_subject@1 = {author, <<"alex"/utf8>>},
    case _assert_subject =:= _assert_subject@1 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"author_committer_split_test"/utf8>>,
                line => 577,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject,
                    start => 17083,
                    'end' => 17097
                    },
                right => #{kind => literal,
                    value => _assert_subject@1,
                    start => 17101,
                    'end' => 17129
                    },
                start => 17076,
                'end' => 17129,
                expression_start => 17083})
    end,
    _assert_subject@2 = erlang:element(3, Witness),
    _assert_subject@3 = {committer, <<"reed"/utf8>>},
    case _assert_subject@2 =:= _assert_subject@3 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"author_committer_split_test"/utf8>>,
                line => 578,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@2,
                    start => 17139,
                    'end' => 17156
                    },
                right => #{kind => literal,
                    value => _assert_subject@3,
                    start => 17160,
                    'end' => 17191
                    },
                start => 17132,
                'end' => 17191,
                expression_start => 17139})
    end,
    _assert_subject@4 = erlang:element(5, Witness),
    _assert_subject@5 = {message, <<"bias execution trace"/utf8>>},
    case _assert_subject@4 =:= _assert_subject@5 of
        true -> nil;
        false -> erlang:error(#{gleam_error => assert,
                message => <<"Assertion failed."/utf8>>,
                file => <<?FILEPATH/utf8>>,
                module => <<"fragmentation_test"/utf8>>,
                function => <<"author_committer_split_test"/utf8>>,
                line => 579,
                kind => binary_operator,
                operator => '==',
                left => #{kind => expression,
                    value => _assert_subject@4,
                    start => 17201,
                    'end' => 17216
                    },
                right => #{kind => literal,
                    value => _assert_subject@5,
                    start => 17220,
                    'end' => 17265
                    },
                start => 17194,
                'end' => 17265,
                expression_start => 17201})
    end.
