//! `frgmnt` — the fragmentation MCP server binary.
//!
//! Renamed from `fragmentation-mcp` in T2 per Alex's directive
//! ("build fragmentation as a frgmnt binary"). Single-word; PATH-
//! friendly; matches the `frgmt-git` neighbour binary's pattern.
//!
//! T2 ships stdio transport + shard sub-tools wired against the
//! `HamiltonScheduler` stub (per `docs/specs/fragmentation-mcp.md`
//! §9 T2). `--http :PORT`, `--repo PATH`, `--budget-mb N`, and
//! `--import-from .git/` are reserved flag shapes for T3/T4; they
//! fail with a helpful message pointing at the tick that lands
//! them.
//!
//! # Usage (T2)
//!
//! ```text
//! frgmnt [--stdio]
//! ```
//!
//! `--stdio` is the default and currently only transport. Reads
//! newline-delimited JSON-RPC 2.0 requests from stdin; writes
//! responses to stdout.

use std::process::ExitCode;

use fragmentation_mcp::Mcp;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut transport: Option<&'static str> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--stdio" => transport = Some("stdio"),
            "--http" => {
                // The HTTP transport is T4 (per fragmentation-mcp.md §9).
                let _port = iter.next();
                eprintln!("frgmnt: --http transport lands in T4; T2 ships stdio only");
                return ExitCode::from(2);
            }
            "--repo" | "--budget-mb" | "--import-from" => {
                // `--budget-mb` here is the SERVER-DEFAULT shard budget; the
                // per-shard budget is named on the `fragmentation.shard.open`
                // tool call. T3 wires the server-default; T4 wires --repo
                // + --import-from for the git crosswalk.
                let _value = iter.next();
                eprintln!(
                    "frgmnt: {arg} is a reserved flag; lands in T3 (default budget) or T4 (crosswalk)"
                );
                return ExitCode::from(2);
            }
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("frgmnt {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("frgmnt: unknown arg `{other}`. Try --help.");
                return ExitCode::from(2);
            }
        }
    }
    let _ = transport.unwrap_or("stdio");

    // tokio runtime: current-thread stays the right call for T2. The
    // MCP wire is serial — one request per line, one response per
    // line — and the dispatch path is CPU-cheap (the scheduler tick
    // is a stub increment). Multi-threaded scheduling is a T3+
    // refinement once tools/call routes into the actual
    // HamiltonScheduler logic per `docs/specs/hamilton-scheduler.md`.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("frgmnt: tokio runtime construction failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mcp = Mcp::new();
    match runtime.block_on(mcp.run_stdio()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("frgmnt: stdio loop terminated with error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "frgmnt {version}\n\
         MCP server for content-addressed agent workflows.\n\
         \n\
         USAGE:\n    \
             frgmnt [--stdio]\n\
         \n\
         FLAGS:\n    \
             --stdio       Use stdio transport (default; only T2 transport)\n    \
             -h, --help    Print this help\n    \
             -V, --version Print version\n\
         \n\
         RESERVED (per docs/specs/fragmentation-mcp.md §9):\n    \
             --http :PORT       HTTP transport (T4)\n    \
             --repo PATH        Repo root for the default shard (T3)\n    \
             --budget-mb N      Server-default shard budget (T3)\n    \
             --import-from .git Crosswalk import (T4)\n",
        version = env!("CARGO_PKG_VERSION")
    );
}
