//! `fragmentation-mcp` — the MCP server binary.
//!
//! T1 ships the stdio transport only. `--http :PORT`, `--repo PATH`,
//! `--budget-mb N`, and `--import-from .git/` are reserved flag
//! shapes per the spec (§9 T1/T4); they fail with a helpful message
//! pointing at the tick that lands them.
//!
//! # Usage (T1)
//!
//! ```text
//! fragmentation-mcp [--stdio]
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
                eprintln!("fragmentation-mcp: --http transport lands in T4; T1 ships stdio only");
                return ExitCode::from(2);
            }
            "--repo" | "--budget-mb" | "--import-from" => {
                // Shard + crosswalk flags land in T2/T4.
                let _value = iter.next();
                eprintln!(
                    "fragmentation-mcp: {arg} is a reserved flag; lands in T2 (shard) or T4 (crosswalk)"
                );
                return ExitCode::from(2);
            }
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("fragmentation-mcp {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("fragmentation-mcp: unknown arg `{other}`. Try --help.");
                return ExitCode::from(2);
            }
        }
    }
    let _ = transport.unwrap_or("stdio");

    // tokio runtime: current-thread is right for T1. The MCP wire is
    // serial — one request per line, one response per line — and the
    // dispatch path is CPU-cheap (stubs only). Multi-threaded
    // scheduling is a T2/T3 refinement once tools/call routes into
    // the HamiltonScheduler's tick.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("fragmentation-mcp: tokio runtime construction failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mcp = Mcp::new();
    match runtime.block_on(mcp.run_stdio()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fragmentation-mcp: stdio loop terminated with error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "fragmentation-mcp {version}\n\
         MCP server for content-addressed agent workflows.\n\
         \n\
         USAGE:\n    \
             fragmentation-mcp [--stdio]\n\
         \n\
         FLAGS:\n    \
             --stdio       Use stdio transport (default; only T1 transport)\n    \
             -h, --help    Print this help\n    \
             -V, --version Print version\n\
         \n\
         RESERVED (per docs/specs/fragmentation-mcp.md §9):\n    \
             --http :PORT       HTTP transport (T4)\n    \
             --repo PATH        Repo root for the shard (T2)\n    \
             --budget-mb N      Shard budget (T2)\n    \
             --import-from .git Crosswalk import (T4)\n",
        version = env!("CARGO_PKG_VERSION")
    );
}
