-module(fragmentation@git).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/fragmentation/git.gleam").
-export([write/2]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("src/fragmentation/git.gleam", 20).
?DOC(
    " Write a fragment to disk under `dir`, named by its content-addressed SHA.\n"
    "\n"
    " Computes the SHA via `fragmentation.hash_fragment`, serializes via\n"
    " `fragmentation.serialize`, then writes to `<dir>/<sha>`.\n"
    " Returns Ok(Nil) on success, Error(FileError) on failure.\n"
    " Idempotent: writing the same fragment twice produces the same file.\n"
).
-spec write(fragmentation:fragment(), binary()) -> {ok, nil} |
    {error, simplifile:file_error()}.
write(Fragment, Dir) ->
    Sha = fragmentation:hash_fragment(Fragment),
    Content = fragmentation:serialize(Fragment),
    Path = <<<<Dir/binary, "/"/utf8>>/binary, Sha/binary>>,
    simplifile:write(Path, Content).
