-module(fragmentation@diff).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/fragmentation/diff.gleam").
-export([summary/1, diff/2]).
-export_type([change/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-type change() :: {added, fragmentation:fragment()} |
    {removed, fragmentation:fragment()} |
    {modified, fragmentation:fragment(), fragmentation:fragment()} |
    {unchanged, fragmentation:fragment()}.

-file("src/fragmentation/diff.gleam", 63).
?DOC(" Summarize a list of changes: #(added, removed, modified, unchanged).\n").
-spec summary(list(change())) -> {integer(), integer(), integer(), integer()}.
summary(Changes) ->
    gleam@list:fold(
        Changes,
        {0, 0, 0, 0},
        fun(Acc, Change) ->
            {A, R, M, U} = Acc,
            case Change of
                {added, _} ->
                    {A + 1, R, M, U};

                {removed, _} ->
                    {A, R + 1, M, U};

                {modified, _, _} ->
                    {A, R, M + 1, U};

                {unchanged, _} ->
                    {A, R, M, U + 1}
            end
        end
    ).

-file("src/fragmentation/diff.gleam", 52).
-spec diff_children(
    list(fragmentation:fragment()),
    list(fragmentation:fragment())
) -> list(change()).
diff_children(Old, New) ->
    case {Old, New} of
        {[], []} ->
            [];

        {[], [N | Rest]} ->
            [{added, N} | diff_children([], Rest)];

        {[O | Rest@1], []} ->
            [{removed, O} | diff_children(Rest@1, [])];

        {[O@1 | Orest], [N@1 | Nrest]} ->
            lists:append(diff(O@1, N@1), diff_children(Orest, Nrest))
    end.

-file("src/fragmentation/diff.gleam", 32).
?DOC(
    " Diff two fragment trees by their roots.\n"
    " Compares structurally: same hash = unchanged, different hash = modified.\n"
    " Children compared positionally.\n"
).
-spec diff(fragmentation:fragment(), fragmentation:fragment()) -> list(change()).
diff(Old, New) ->
    case fragmentation:hash_fragment(Old) =:= fragmentation:hash_fragment(New) of
        true ->
            [{unchanged, Old}];

        false ->
            diff_fragments(Old, New)
    end.

-file("src/fragmentation/diff.gleam", 39).
-spec diff_fragments(fragmentation:fragment(), fragmentation:fragment()) -> list(change()).
diff_fragments(Old, New) ->
    Old_children = fragmentation:children(Old),
    New_children = fragmentation:children(New),
    Root_change = [{modified, Old, New}],
    Child_changes = diff_children(Old_children, New_children),
    lists:append(Root_change, Child_changes).
