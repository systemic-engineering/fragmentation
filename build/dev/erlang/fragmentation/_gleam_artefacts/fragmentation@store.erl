-module(fragmentation@store).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/fragmentation/store.gleam").
-export([new/0, put/2, get/2, has/2, size/1, merge/2, keys/1]).
-export_type([store/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-opaque store() :: {store, gleam@dict:dict(binary(), fragmentation:fragment())}.

-file("src/fragmentation/store.gleam", 22).
?DOC(" Create an empty store.\n").
-spec new() -> store().
new() ->
    {store, maps:new()}.

-file("src/fragmentation/store.gleam", 31).
?DOC(" Insert a fragment by its self-ref SHA.\n").
-spec put(store(), fragmentation:fragment()) -> store().
put(Store, Frag) ->
    {ref, {sha, Key}, _} = fragmentation:self_ref(Frag),
    {store, gleam@dict:insert(erlang:element(2, Store), Key, Frag)}.

-file("src/fragmentation/store.gleam", 38).
?DOC(" Look up a fragment by SHA.\n").
-spec get(store(), fragmentation:sha()) -> {ok, fragmentation:fragment()} |
    {error, nil}.
get(Store, S) ->
    {sha, Key} = S,
    gleam_stdlib:map_get(erlang:element(2, Store), Key).

-file("src/fragmentation/store.gleam", 44).
?DOC(" Check if a fragment exists.\n").
-spec has(store(), fragmentation:sha()) -> boolean().
has(Store, S) ->
    {sha, Key} = S,
    gleam@dict:has_key(erlang:element(2, Store), Key).

-file("src/fragmentation/store.gleam", 50).
?DOC(" Count fragments in the store.\n").
-spec size(store()) -> integer().
size(Store) ->
    maps:size(erlang:element(2, Store)).

-file("src/fragmentation/store.gleam", 55).
?DOC(" Merge two stores. Same SHA = same content.\n").
-spec merge(store(), store()) -> store().
merge(A, B) ->
    {store, maps:merge(erlang:element(2, A), erlang:element(2, B))}.

-file("src/fragmentation/store.gleam", 60).
?DOC(" List all SHAs in the store.\n").
-spec keys(store()) -> list(fragmentation:sha()).
keys(Store) ->
    _pipe = maps:keys(erlang:element(2, Store)),
    gleam@list:map(_pipe, fun(K) -> {sha, K} end).
