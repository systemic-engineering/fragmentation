-module(fragmentation_ffi).
-export([sha512/1]).

sha512(Data) ->
    Hash = crypto:hash(sha512, Data),
    list_to_binary(
        lists:flatten(
            [io_lib:format("~2.16.0b", [B]) || <<B>> <= Hash]
        )
    ).
