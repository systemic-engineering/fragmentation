-module(fragmentation).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/fragmentation.gleam").
-export([sha/1, ref/2, author/1, committer/1, timestamp/1, message/1, witnessed/4, shard/3, fragment/4, serialize_witnessed/1, serialize_ref/1, serialize/1, self_ref/1, self_witnessed/1, data/1, children/1, is_shard/1, is_fragment/1, hash/1, hash_fragment/1]).
-export_type([sha/0, ref/0, author/0, committer/0, timestamp/0, message/0, witnessed/0, fragment/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-type sha() :: {sha, binary()}.

-type ref() :: {ref, sha(), binary()}.

-type author() :: {author, binary()}.

-type committer() :: {committer, binary()}.

-type timestamp() :: {timestamp, binary()}.

-type message() :: {message, binary()}.

-type witnessed() :: {witnessed, author(), committer(), timestamp(), message()}.

-type fragment() :: {shard, ref(), witnessed(), binary()} |
    {fragment, ref(), witnessed(), binary(), list(fragment())}.

-file("src/fragmentation.gleam", 75).
?DOC(" Create a SHA from a raw string.\n").
-spec sha(binary()) -> sha().
sha(Value) ->
    {sha, Value}.

-file("src/fragmentation.gleam", 80).
?DOC(" Create a reference.\n").
-spec ref(sha(), binary()) -> ref().
ref(S, Label) ->
    {ref, S, Label}.

-file("src/fragmentation.gleam", 85).
?DOC(" Create an author value.\n").
-spec author(binary()) -> author().
author(Value) ->
    {author, Value}.

-file("src/fragmentation.gleam", 90).
?DOC(" Create a committer value.\n").
-spec committer(binary()) -> committer().
committer(Value) ->
    {committer, Value}.

-file("src/fragmentation.gleam", 95).
?DOC(" Create a timestamp value.\n").
-spec timestamp(binary()) -> timestamp().
timestamp(Value) ->
    {timestamp, Value}.

-file("src/fragmentation.gleam", 100).
?DOC(" Create a message value.\n").
-spec message(binary()) -> message().
message(Value) ->
    {message, Value}.

-file("src/fragmentation.gleam", 105).
?DOC(" Create a witness record.\n").
-spec witnessed(author(), committer(), timestamp(), message()) -> witnessed().
witnessed(A, C, Ts, Msg) ->
    {witnessed, A, C, Ts, Msg}.

-file("src/fragmentation.gleam", 115).
?DOC(" Create a shard. Terminal fragment.\n").
-spec shard(ref(), witnessed(), binary()) -> fragment().
shard(Ref, Witnessed, Data) ->
    {shard, Ref, Witnessed, Data}.

-file("src/fragmentation.gleam", 120).
?DOC(" Create a fragment. Self-similar, contains other fragments.\n").
-spec fragment(ref(), witnessed(), binary(), list(fragment())) -> fragment().
fragment(Ref, Witnessed, Data, Fragments) ->
    {fragment, Ref, Witnessed, Data, Fragments}.

-file("src/fragmentation.gleam", 139).
?DOC(" Deterministic canonical serialization of a witness record.\n").
-spec serialize_witnessed(witnessed()) -> binary().
serialize_witnessed(M) ->
    {author, A} = erlang:element(2, M),
    {committer, C} = erlang:element(3, M),
    {timestamp, Ts} = erlang:element(4, M),
    {message, Msg} = erlang:element(5, M),
    <<<<<<<<<<<<<<"author:"/utf8, A/binary>>/binary, "\ncommitter:"/utf8>>/binary,
                        C/binary>>/binary,
                    "\ntimestamp:"/utf8>>/binary,
                Ts/binary>>/binary,
            "\nmessage:"/utf8>>/binary,
        Msg/binary>>.

-file("src/fragmentation.gleam", 155).
?DOC(" Deterministic canonical serialization of a ref.\n").
-spec serialize_ref(ref()) -> binary().
serialize_ref(R) ->
    {ref, {sha, S}, Label} = R,
    <<<<<<"ref:"/utf8, S/binary>>/binary, ":"/utf8>>/binary, Label/binary>>.

-file("src/fragmentation.gleam", 161).
?DOC(" Deterministic canonical serialization of a fragment.\n").
-spec serialize(fragment()) -> binary().
serialize(Frag) ->
    case Frag of
        {shard, R, M, D} ->
            <<<<<<<<<<"shard\n"/utf8, (serialize_ref(R))/binary>>/binary,
                            "\n"/utf8>>/binary,
                        (serialize_witnessed(M))/binary>>/binary,
                    "\ndata:"/utf8>>/binary,
                D/binary>>;

        {fragment, R@1, M@1, D@1, Fs} ->
            <<<<<<<<<<<<<<<<"fragment\n"/utf8, (serialize_ref(R@1))/binary>>/binary,
                                        "\n"/utf8>>/binary,
                                    (serialize_witnessed(M@1))/binary>>/binary,
                                "\ndata:"/utf8>>/binary,
                            D@1/binary>>/binary,
                        "\nfragments:["/utf8>>/binary,
                    (begin
                        _pipe = Fs,
                        _pipe@1 = gleam@list:map(
                            _pipe,
                            fun(F) -> serialize(F) end
                        ),
                        gleam@string:join(_pipe@1, <<","/utf8>>)
                    end)/binary>>/binary,
                "]"/utf8>>
    end.

-file("src/fragmentation.gleam", 197).
?DOC(" Get the ref (self-address) of a fragment.\n").
-spec self_ref(fragment()) -> ref().
self_ref(Frag) ->
    case Frag of
        {shard, R, _, _} ->
            R;

        {fragment, R@1, _, _, _} ->
            R@1
    end.

-file("src/fragmentation.gleam", 205).
?DOC(" Get the witness record of a fragment.\n").
-spec self_witnessed(fragment()) -> witnessed().
self_witnessed(Frag) ->
    case Frag of
        {shard, _, W, _} ->
            W;

        {fragment, _, W@1, _, _} ->
            W@1
    end.

-file("src/fragmentation.gleam", 213).
?DOC(" Get the data from a fragment.\n").
-spec data(fragment()) -> binary().
data(Frag) ->
    case Frag of
        {shard, _, _, D} ->
            D;

        {fragment, _, _, D@1, _} ->
            D@1
    end.

-file("src/fragmentation.gleam", 221).
?DOC(" Get child fragments. Shards have none.\n").
-spec children(fragment()) -> list(fragment()).
children(Frag) ->
    case Frag of
        {shard, _, _, _} ->
            [];

        {fragment, _, _, _, Fs} ->
            Fs
    end.

-file("src/fragmentation.gleam", 229).
?DOC(" Check if a fragment is a shard.\n").
-spec is_shard(fragment()) -> boolean().
is_shard(Frag) ->
    case Frag of
        {shard, _, _, _} ->
            true;

        {fragment, _, _, _, _} ->
            false
    end.

-file("src/fragmentation.gleam", 237).
?DOC(" Check if a fragment is a fragment (non-terminal).\n").
-spec is_fragment(fragment()) -> boolean().
is_fragment(Frag) ->
    case Frag of
        {shard, _, _, _} ->
            false;

        {fragment, _, _, _, _} ->
            true
    end.

-file("src/fragmentation.gleam", 134).
?DOC(" Raw SHA-256 hash of a string.\n").
-spec hash(binary()) -> sha().
hash(Data) ->
    {sha, fragmentation_ffi:sha256(Data)}.

-file("src/fragmentation.gleam", 188).
?DOC(" Content-address a fragment: SHA-256 of its canonical serialization.\n").
-spec hash_fragment(fragment()) -> binary().
hash_fragment(Frag) ->
    fragmentation_ffi:sha256(serialize(Frag)).
