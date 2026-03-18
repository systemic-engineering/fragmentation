-module(fragmentation_ffi).
-export([sha256/1]).

sha256(Data) ->
    Hash = crypto:hash(sha256, Data),
    list_to_binary(
        lists:flatten(
            [io_lib:format("~2.16.0b", [B]) || <<B>> <= Hash]
        )
    ).
