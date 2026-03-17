-module(fragmentation_git_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/fragmentation_git_test.gleam").
-export([write_fragment_creates_file_test/0, write_fragment_idempotent_test/0, write_two_fragments_test/0]).

-file("test/fragmentation_git_test.gleam", 10).
-spec test_witnessed() -> fragmentation:witnessed().
test_witnessed() ->
    fragmentation:witnessed(
        fragmentation:author(<<"alex"/utf8>>),
        fragmentation:committer(<<"reed"/utf8>>),
        fragmentation:timestamp(<<"2026-03-01T00:00:00Z"/utf8>>),
        fragmentation:message(<<"test"/utf8>>)
    ).

-file("test/fragmentation_git_test.gleam", 19).
-spec make_shard(binary()) -> fragmentation:fragment().
make_shard(Data) ->
    R = fragmentation:ref(fragmentation:hash(Data), <<"self"/utf8>>),
    fragmentation:shard(R, test_witnessed(), Data).

-file("test/fragmentation_git_test.gleam", 30).
-spec write_fragment_creates_file_test() -> nil.
write_fragment_creates_file_test() ->
    Dir = <<"/tmp/fragmentation_git_test_write"/utf8>>,
    _ = simplifile_erl:create_directory(Dir),
    Frag = make_shard(<<"hello-world"/utf8>>),
    Sha = fragmentation:hash_fragment(Frag),
    Result = fragmentation@git:write(Frag, Dir),
    _pipe = Result,
    gleeunit@should:be_ok(_pipe),
    Path = <<<<Dir/binary, "/"/utf8>>/binary, Sha/binary>>,
    _pipe@1 = simplifile_erl:is_file(Path),
    gleeunit@should:equal(_pipe@1, {ok, true}).

-file("test/fragmentation_git_test.gleam", 49).
-spec write_fragment_idempotent_test() -> nil.
write_fragment_idempotent_test() ->
    Dir = <<"/tmp/fragmentation_git_test_idempotent"/utf8>>,
    _ = simplifile_erl:create_directory(Dir),
    Frag = make_shard(<<"idempotent-shard"/utf8>>),
    Sha = fragmentation:hash_fragment(Frag),
    R1 = fragmentation@git:write(Frag, Dir),
    R2 = fragmentation@git:write(Frag, Dir),
    _pipe = R1,
    gleeunit@should:be_ok(_pipe),
    _pipe@1 = R2,
    gleeunit@should:be_ok(_pipe@1),
    Path = <<<<Dir/binary, "/"/utf8>>/binary, Sha/binary>>,
    _pipe@2 = simplifile_erl:is_file(Path),
    gleeunit@should:equal(_pipe@2, {ok, true}).

-file("test/fragmentation_git_test.gleam", 72).
-spec write_two_fragments_test() -> nil.
write_two_fragments_test() ->
    Dir = <<"/tmp/fragmentation_git_test_two"/utf8>>,
    _ = simplifile_erl:create_directory(Dir),
    Frag_a = make_shard(<<"fragment-alpha"/utf8>>),
    Frag_b = make_shard(<<"fragment-beta"/utf8>>),
    Sha_a = fragmentation:hash_fragment(Frag_a),
    Sha_b = fragmentation:hash_fragment(Frag_b),
    _pipe = fragmentation@git:write(Frag_a, Dir),
    gleeunit@should:be_ok(_pipe),
    _pipe@1 = fragmentation@git:write(Frag_b, Dir),
    gleeunit@should:be_ok(_pipe@1),
    _pipe@2 = Sha_a,
    gleeunit@should:not_equal(_pipe@2, Sha_b),
    _pipe@3 = simplifile_erl:is_file(
        <<<<Dir/binary, "/"/utf8>>/binary, Sha_a/binary>>
    ),
    gleeunit@should:equal(_pipe@3, {ok, true}),
    _pipe@4 = simplifile_erl:is_file(
        <<<<Dir/binary, "/"/utf8>>/binary, Sha_b/binary>>
    ),
    gleeunit@should:equal(_pipe@4, {ok, true}).
