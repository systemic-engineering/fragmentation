-module(fragmentation@encoding).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/fragmentation/encoding.gleam").
-export([encode_char/2, encode_word/2, encode_sentence/2, decode/1, encode_paragraph/2, encode/2, ingest/3]).
-export_type([decode_error/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-type decode_error() :: {unknown_label, binary()}.

-file("src/fragmentation/encoding.gleam", 18).
?DOC(
    " Hash data with its label namespace to avoid cross-level collisions.\n"
    " A char \"a\" and word \"a\" must have different SHAs in the store.\n"
).
-spec labeled_hash(binary(), binary()) -> fragmentation:sha().
labeled_hash(Label, Data) ->
    fragmentation:hash(<<<<Label/binary, ":"/utf8>>/binary, Data/binary>>).

-file("src/fragmentation/encoding.gleam", 23).
?DOC(" Encode a single character as a Shard.\n").
-spec encode_char(binary(), fragmentation:witnessed()) -> fragmentation:fragment().
encode_char(Char, Witness) ->
    Label = <<"utf8/"/utf8, Char/binary>>,
    Sha = labeled_hash(Label, Char),
    R = fragmentation:ref(Sha, Label),
    fragmentation:shard(R, Witness, Char).

-file("src/fragmentation/encoding.gleam", 31).
?DOC(" Encode a word as a Fragment of character Shards.\n").
-spec encode_word(binary(), fragmentation:witnessed()) -> fragmentation:fragment().
encode_word(Word, Witness) ->
    Chars = begin
        _pipe = gleam@string:to_graphemes(Word),
        gleam@list:map(_pipe, fun(C) -> encode_char(C, Witness) end)
    end,
    Label = <<"token/"/utf8, Word/binary>>,
    Sha = labeled_hash(Label, Word),
    R = fragmentation:ref(Sha, Label),
    fragmentation:fragment(R, Witness, Word, Chars).

-file("src/fragmentation/encoding.gleam", 42).
?DOC(" Encode a sentence as a Fragment of word Fragments.\n").
-spec encode_sentence(binary(), fragmentation:witnessed()) -> fragmentation:fragment().
encode_sentence(Text, Witness) ->
    Words = begin
        _pipe = gleam@string:split(Text, <<" "/utf8>>),
        _pipe@1 = gleam@list:filter(_pipe, fun(W) -> W /= <<""/utf8>> end),
        gleam@list:map(_pipe@1, fun(W@1) -> encode_word(W@1, Witness) end)
    end,
    Sha = labeled_hash(<<"sentence"/utf8>>, Text),
    R = fragmentation:ref(Sha, <<"sentence"/utf8>>),
    fragmentation:fragment(R, Witness, Text, Words).

-file("src/fragmentation/encoding.gleam", 85).
?DOC(" Decode a Fragment tree back to text.\n").
-spec decode(fragmentation:fragment()) -> {ok, binary()} |
    {error, decode_error()}.
decode(Fragment) ->
    {ok, fragmentation:data(Fragment)}.

-file("src/fragmentation/encoding.gleam", 101).
-spec do_split_sentences(list(binary()), binary(), list(binary())) -> list(binary()).
do_split_sentences(Chars, Current, Acc) ->
    case Chars of
        [] ->
            case Current of
                <<""/utf8>> ->
                    Acc;

                _ ->
                    [Current | Acc]
            end;

        [<<"."/utf8>>, <<" "/utf8>> | Rest] ->
            do_split_sentences(
                Rest,
                <<""/utf8>>,
                [<<Current/binary, "."/utf8>> | Acc]
            );

        [<<"!"/utf8>>, <<" "/utf8>> | Rest@1] ->
            do_split_sentences(
                Rest@1,
                <<""/utf8>>,
                [<<Current/binary, "!"/utf8>> | Acc]
            );

        [<<"?"/utf8>>, <<" "/utf8>> | Rest@2] ->
            do_split_sentences(
                Rest@2,
                <<""/utf8>>,
                [<<Current/binary, "?"/utf8>> | Acc]
            );

        [C | Rest@3] ->
            do_split_sentences(Rest@3, <<Current/binary, C/binary>>, Acc)
    end.

-file("src/fragmentation/encoding.gleam", 95).
?DOC(
    " Split text into sentences on \". \", \"! \", \"? \" boundaries.\n"
    " Punctuation stays with the preceding sentence.\n"
).
-spec split_sentences(binary()) -> list(binary()).
split_sentences(Text) ->
    _pipe = gleam@string:to_graphemes(Text),
    _pipe@1 = do_split_sentences(_pipe, <<""/utf8>>, []),
    lists:reverse(_pipe@1).

-file("src/fragmentation/encoding.gleam", 53).
?DOC(" Encode a paragraph as a Fragment of sentence Fragments.\n").
-spec encode_paragraph(binary(), fragmentation:witnessed()) -> fragmentation:fragment().
encode_paragraph(Text, Witness) ->
    Sentences = begin
        _pipe = split_sentences(Text),
        _pipe@1 = gleam@list:filter(_pipe, fun(S) -> S /= <<""/utf8>> end),
        gleam@list:map(_pipe@1, fun(S@1) -> encode_sentence(S@1, Witness) end)
    end,
    Sha = labeled_hash(<<"paragraph"/utf8>>, Text),
    R = fragmentation:ref(Sha, <<"paragraph"/utf8>>),
    fragmentation:fragment(R, Witness, Text, Sentences).

-file("src/fragmentation/encoding.gleam", 65).
?DOC(
    " Encode full text as a document Fragment.\n"
    " Splits on double newlines into paragraphs.\n"
).
-spec encode(binary(), fragmentation:witnessed()) -> fragmentation:fragment().
encode(Text, Witness) ->
    Paragraphs = begin
        _pipe = gleam@string:split(Text, <<"\n\n"/utf8>>),
        _pipe@1 = gleam@list:filter(_pipe, fun(P) -> P /= <<""/utf8>> end),
        gleam@list:map(_pipe@1, fun(P@1) -> encode_paragraph(P@1, Witness) end)
    end,
    Sha = labeled_hash(<<"document"/utf8>>, Text),
    R = fragmentation:ref(Sha, <<"document"/utf8>>),
    fragmentation:fragment(R, Witness, Text, Paragraphs).

-file("src/fragmentation/encoding.gleam", 76).
?DOC(" Encode and store, returning root Fragment + updated Store (deduped).\n").
-spec ingest(binary(), fragmentation:witnessed(), fragmentation@store:store()) -> {fragmentation:fragment(),
    fragmentation@store:store()}.
ingest(Text, Witness, S) ->
    Root = encode(Text, Witness),
    Updated = begin
        _pipe = fragmentation@walk:collect(Root),
        gleam@list:fold(
            _pipe,
            S,
            fun(Acc, Frag) -> fragmentation@store:put(Acc, Frag) end
        )
    end,
    {Root, Updated}.
