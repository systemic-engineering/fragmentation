-record(fragment, {
    ref :: fragmentation:ref(),
    witnessed :: fragmentation:witnessed(),
    data :: binary(),
    fragments :: list(fragmentation:fragment())
}).
