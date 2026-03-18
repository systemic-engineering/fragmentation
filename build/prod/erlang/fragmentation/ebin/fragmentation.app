{application, fragmentation, [
    {vsn, "0.1.0"},
    {applications, [gleam_stdlib]},
    {description, "fragmentation — encoded possibility space. content-addressed, arbitrary depth, circular-reflexive. reality for git."},
    {modules, [fragmentation,
               fragmentation@@main,
               fragmentation@diff,
               fragmentation@store,
               fragmentation@walk,
               fragmentation_ffi]},
    {registered, []}
]}.
