-module(fragmentation@walk).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/fragmentation/walk.gleam").
-export([collect/1, fold/3, depth/1, find/2]).
-export_type([visitor/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-type visitor(TX) :: {continue, TX} | {stop, TX}.

-file("src/fragmentation/walk.gleam", 31).
-spec do_collect(fragmentation:fragment(), list(fragmentation:fragment())) -> list(fragmentation:fragment()).
do_collect(Frag, Acc) ->
    Acc@1 = [Frag | Acc],
    case fragmentation:children(Frag) of
        [] ->
            Acc@1;

        Children ->
            gleam@list:fold(
                Children,
                Acc@1,
                fun(A, Child) -> do_collect(Child, A) end
            )
    end.

-file("src/fragmentation/walk.gleam", 26).
?DOC(" Collect all fragments in a tree, depth-first.\n").
-spec collect(fragmentation:fragment()) -> list(fragmentation:fragment()).
collect(Root) ->
    _pipe = do_collect(Root, []),
    lists:reverse(_pipe).

-file("src/fragmentation/walk.gleam", 44).
-spec do_fold(
    fragmentation:fragment(),
    UD,
    fun((UD, fragmentation:fragment()) -> visitor(UD))
) -> UD.
do_fold(Frag, Acc, F) ->
    case F(Acc, Frag) of
        {stop, Result} ->
            Result;

        {continue, Result@1} ->
            case fragmentation:children(Frag) of
                [] ->
                    Result@1;

                Children ->
                    gleam@list:fold(
                        Children,
                        Result@1,
                        fun(A, Child) -> do_fold(Child, A, F) end
                    )
            end
    end.

-file("src/fragmentation/walk.gleam", 40).
?DOC(" Fold over all fragments in a tree, depth-first.\n").
-spec fold(
    fragmentation:fragment(),
    UB,
    fun((UB, fragmentation:fragment()) -> visitor(UB))
) -> UB.
fold(Root, Acc, F) ->
    do_fold(Root, Acc, F).

-file("src/fragmentation/walk.gleam", 57).
?DOC(" Get the depth of a fragment tree.\n").
-spec depth(fragmentation:fragment()) -> integer().
depth(Root) ->
    case fragmentation:children(Root) of
        [] ->
            0;

        Children ->
            Max_child_depth = begin
                _pipe = Children,
                _pipe@1 = gleam@list:map(_pipe, fun(Child) -> depth(Child) end),
                gleam@list:fold(_pipe@1, 0, fun(Best, D) -> case D > Best of
                            true ->
                                D;

                            false ->
                                Best
                        end end)
            end,
            1 + Max_child_depth
    end.

-file("src/fragmentation/walk.gleam", 88).
-spec do_find(
    list(fragmentation:fragment()),
    fun((fragmentation:fragment()) -> boolean())
) -> {ok, fragmentation:fragment()} | {error, nil}.
do_find(Fragments, Predicate) ->
    case Fragments of
        [] ->
            {error, nil};

        [First | Rest] ->
            case find(First, Predicate) of
                {ok, Found} ->
                    {ok, Found};

                {error, nil} ->
                    do_find(Rest, Predicate)
            end
    end.

-file("src/fragmentation/walk.gleam", 76).
?DOC(" Find the first fragment matching a predicate, depth-first.\n").
-spec find(
    fragmentation:fragment(),
    fun((fragmentation:fragment()) -> boolean())
) -> {ok, fragmentation:fragment()} | {error, nil}.
find(Root, Predicate) ->
    case Predicate(Root) of
        true ->
            {ok, Root};

        false ->
            _pipe = fragmentation:children(Root),
            do_find(_pipe, Predicate)
    end.
