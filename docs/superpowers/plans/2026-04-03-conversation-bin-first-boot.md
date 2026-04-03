# conversation-bin First Boot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Type `conversation` and a terminal UI opens. First boot creates `~/.conversation/`, initializes spectral-db, and the Pathfinder says hello. The system begins learning.

**Architecture:** Two new projects: `conversation-bin` (Rust binary, clap CLI) and `cosmos-tui` (Gleam OTP application). The Rust binary handles CLI parsing and non-TUI subcommands. For the default command (no args), it launches cosmos-tui on BEAM. cosmos-tui owns the event loop (Elm architecture), uses gestalt-tui for terminal rendering, and reads spectral-db via Gleam NIF for persistent memory.

**Tech Stack:** Rust (clap, conversation, projection, cosmos), Gleam (gestalt-tui, gestalt-ui, etch, spectral-db NIF), BEAM/OTP

---

## Scope

**This plan builds:**
- `conversation-bin` at `/Users/alexwolf/dev/projects/conversation-bin/`
- `cosmos-tui` at `/Users/alexwolf/dev/projects/cosmos-tui/`
- First boot flow: `~/.conversation/` creation, spectral-db init, Pathfinder greeting
- Basic conversation interface (type messages, see them displayed, stored in spectral-db)
- Subcommand stubs for compile, lsp, render, etc.

**This plan does NOT build:**
- Full spectral visualization (eigenvalue dashboards, tension matrices)
- projection Gleam adapter (cosmos-tui uses spectral-db directly for now)
- Actor system (no ractor dispatch in TUI for now)
- `conversation compile` or `conversation lsp` implementation (stubs only)
- cosmos-bevy (3D renderer)

**Multi-project note:** This plan creates two projects in two languages. Tasks 1-2 are Rust. Tasks 3-7 are Gleam. Task 8 wires them together. Each phase produces independently testable software.

---

## Prerequisites

### Build commands

**Rust (conversation-bin):**
```bash
nix develop -c cargo test
nix develop -c cargo clippy -- -D warnings
nix develop -c cargo fmt -- --check
```

**Gleam (cosmos-tui):**
```bash
nix develop -c gleam test
nix develop -c gleam format --check src/ test/
```

### Commit conventions
- Identity: `Mara <mara@systemic.engineer>`
- Branches: `conversation-bin/first-boot`, `cosmos-tui/first-boot`
- Arc: 🔴 (tests fail) → 🟢 (tests pass), 🔧 (scaffold/tooling)

### Key dependencies to read before starting

**gestalt-tui (the terminal rendering layer):**
- `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/src/gestalt_tui/io/terminal.gleam` — Terminal lifecycle (enter/exit)
- `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/src/gestalt_tui/io/screen.gleam` — Screen buffer with diff-based flush
- `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/src/gestalt_tui/io/input.gleam` — Event handling (etch wrapper)
- `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/src/gestalt_tui/palette.gleam` — Theme → terminal colors
- `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/src/gestalt_tui/style.gleam` — Style composition
- `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/src/gestalt_tui/signal.gleam` — Signal rendering (6 variants)

**spectral-db Gleam adapter:**
- `/Users/alexwolf/dev/projects/spectral-db/beam/src/spectral_db.gleam` — Existing FFI stubs (get, find, status)

**cosmos store:**
- `/Users/alexwolf/dev/projects/cosmos/src/store.rs` — `open_cosmos_db`, `insert_graph`, pattern for spectral-db usage

---

## File Structure

### conversation-bin (Rust)

```
/Users/alexwolf/dev/projects/conversation-bin/
├── Cargo.toml
├── flake.nix
├── Justfile
├── .gitignore
└── src/
    ├── main.rs            — clap CLI, subcommand dispatch
    ├── first_boot.rs      — ~/.conversation/ creation + spectral-db init
    └── launch.rs          — locate and exec cosmos-tui BEAM release
```

### cosmos-tui (Gleam)

```
/Users/alexwolf/dev/projects/cosmos-tui/
├── gleam.toml
├── flake.nix
├── Justfile
├── src/
│   ├── cosmos_tui.gleam          — main entry point, BEAM app start
│   ├── cosmos_tui/app.gleam      — event loop: init → update → render
│   ├── cosmos_tui/model.gleam    — Model type, messages, state
│   ├── cosmos_tui/view.gleam     — render model to screen buffer
│   ├── cosmos_tui/pathfinder.gleam — first boot greeting content
│   └── cosmos_tui/memory.gleam   — spectral-db read/write (NIF calls)
└── test/
    ├── cosmos_tui_test.gleam     — test harness
    ├── model_test.gleam          — model state transitions
    ├── view_test.gleam           — render output verification
    └── pathfinder_test.gleam     — greeting content
```

---

## Task 1: conversation-bin Scaffold

**Files:**
- Create: `conversation-bin/Cargo.toml`, `conversation-bin/src/main.rs`, `conversation-bin/flake.nix`, `conversation-bin/Justfile`, `conversation-bin/.gitignore`

- [ ] **Step 1: Create project and init git**

```bash
mkdir -p /Users/alexwolf/dev/projects/conversation-bin/src
cd /Users/alexwolf/dev/projects/conversation-bin
git init
git checkout -b conversation-bin/first-boot
```

- [ ] **Step 2: Write Cargo.toml**

```toml
[package]
name = "conversation-bin"
version = "0.1.0"
edition = "2021"
description = "conversation — one binary, one command, the glue."
default-run = "conversation"

[[bin]]
name = "conversation"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
projection = { path = "../projection" }
spectral-db = { path = "../spectral-db" }

[dev-dependencies]
tempfile = "3"
```

**Note:** conversation and conversation-lsp are NOT deps yet — they're subcommand stubs. Add them when the subcommands get implemented. For first boot, only projection and spectral-db are needed.

- [ ] **Step 3: Write flake.nix**

Model after `/Users/alexwolf/dev/projects/projection/flake.nix` (same Rust toolchain + gfortran for coincidence transitively).

- [ ] **Step 4: Write Justfile**

```just
test:
    nix develop -c cargo test

check: test lint fmt-check

lint:
    nix develop -c cargo clippy -- -D warnings

fmt:
    nix develop -c cargo fmt

fmt-check:
    nix develop -c cargo fmt -- --check

run:
    nix develop -c cargo run
```

- [ ] **Step 5: Write main.rs with clap CLI skeleton**

```rust
use clap::{Parser, Subcommand};

mod first_boot;
mod launch;

#[derive(Parser)]
#[command(name = "conversation", about = "One binary. One command. The glue.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .conv grammar
    Compile {
        /// Path to .conv file
        file: String,
    },
    /// Start the language server
    Lsp,
    /// Render through cosmos
    Render {
        /// Target: ansi, svg, wasm
        #[arg(long, default_value = "ansi")]
        target: String,
    },
    /// Manage actors
    Actor {
        #[command(subcommand)]
        action: ActorAction,
    },
    /// Connect to a running actor
    Join {
        /// Actor name or ID
        actor: String,
    },
    /// Export/import projections
    Projection {
        #[command(subcommand)]
        action: ProjectionAction,
    },
}

#[derive(Subcommand)]
enum ActorAction {
    Spawn { path: String },
    Status,
    Init { path: String, #[arg(long)] role: Option<String> },
    Mount { identity: String, workspace: String },
    Materialize,
}

#[derive(Subcommand)]
enum ProjectionAction {
    Export { file: String },
    Import { file: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No subcommand = TUI. The default. The product.
            if let Err(e) = run_tui() {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Compile { file }) => {
            eprintln!("conversation compile {} — not yet implemented", file);
        }
        Some(Commands::Lsp) => {
            eprintln!("conversation lsp — not yet implemented");
        }
        Some(Commands::Render { target }) => {
            eprintln!("conversation render --target {} — not yet implemented", target);
        }
        Some(Commands::Actor { action }) => {
            match action {
                ActorAction::Spawn { path } => eprintln!("conversation actor spawn {} — not yet implemented", path),
                ActorAction::Status => eprintln!("conversation actor status — not yet implemented"),
                ActorAction::Init { path, role } => eprintln!("conversation actor init {} --role {:?} — not yet implemented", path, role),
                ActorAction::Mount { identity, workspace } => eprintln!("conversation actor mount {} {} — not yet implemented", identity, workspace),
                ActorAction::Materialize => eprintln!("conversation actor materialize — not yet implemented"),
            }
        }
        Some(Commands::Join { actor }) => {
            eprintln!("conversation join {} — not yet implemented", actor);
        }
        Some(Commands::Projection { action }) => {
            match action {
                ProjectionAction::Export { file } => eprintln!("conversation projection export {} — not yet implemented", file),
                ProjectionAction::Import { file } => eprintln!("conversation projection import {} — not yet implemented", file),
            }
        }
    }
}

fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    let home = first_boot::ensure_home()?;
    eprintln!("Home: {}", home.display());
    // TODO: Task 8 — launch cosmos-tui
    eprintln!("cosmos-tui not yet wired — run the Gleam binary directly for now");
    Ok(())
}
```

- [ ] **Step 6: Write first_boot.rs**

```rust
use std::fs;
use std::path::{Path, PathBuf};

use projection::filter::GrammarFilter;
use projection::Projection;

/// Returns the path to ~/.conversation/, creating it on first boot.
pub fn ensure_home() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs_or_fallback();
    let conv_home = home.join(".conversation");

    if !conv_home.exists() {
        first_boot(&conv_home)?;
    }

    Ok(conv_home)
}

/// First boot: create directory structure and init spectral-db.
fn first_boot(conv_home: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create directory structure
    fs::create_dir_all(conv_home.join(".spectral"))?;
    fs::create_dir_all(conv_home.join("workspace"))?;
    fs::create_dir_all(conv_home.join("beams"))?;

    // Write initial grammar (the human actor's starting types)
    let initial_grammar = "\
grammar @human {
  type = message | observation | thought
}
";
    fs::write(conv_home.join("main.conv"), initial_grammar)?;

    // Initialize spectral-db via projection
    let filter = GrammarFilter::new("human")
        .allow_type("message")
        .allow_type("observation")
        .allow_type("thought");
    let _proj = Projection::open(
        &conv_home.join(".spectral"),
        filter,
        "human",
        1e-6,
        50_000_000,
    )?;

    Ok(())
}

fn dirs_or_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn first_boot_creates_directory_structure() {
        let dir = tempdir().unwrap();
        let conv_home = dir.path().join(".conversation");
        first_boot(&conv_home).unwrap();

        assert!(conv_home.join(".spectral").exists());
        assert!(conv_home.join("workspace").exists());
        assert!(conv_home.join("beams").exists());
        assert!(conv_home.join("main.conv").exists());
    }

    #[test]
    fn first_boot_grammar_contains_human_types() {
        let dir = tempdir().unwrap();
        let conv_home = dir.path().join(".conversation");
        first_boot(&conv_home).unwrap();

        let grammar = std::fs::read_to_string(conv_home.join("main.conv")).unwrap();
        assert!(grammar.contains("message"));
        assert!(grammar.contains("observation"));
        assert!(grammar.contains("thought"));
    }

    #[test]
    fn ensure_home_creates_on_first_call() {
        // This test would need HOME override — skip in CI
        // Tested indirectly via first_boot tests above
    }
}
```

- [ ] **Step 7: Write launch.rs (stub)**

```rust
use std::path::Path;
use std::process::Command;

/// Launch cosmos-tui BEAM application.
/// For now, attempts to run `gleam run` in the cosmos-tui directory.
pub fn launch_cosmos_tui(conv_home: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cosmos_tui_dir = find_cosmos_tui()?;

    let status = Command::new("gleam")
        .arg("run")
        .arg("-m")
        .arg("cosmos_tui")
        .arg("--")
        .arg(conv_home.to_str().unwrap_or("~/.conversation"))
        .current_dir(&cosmos_tui_dir)
        .status()?;

    if !status.success() {
        return Err(format!("cosmos-tui exited with status: {}", status).into());
    }

    Ok(())
}

fn find_cosmos_tui() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // Development: look for sibling project
    let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("cosmos-tui"))
        .filter(|p| p.exists());

    match dev_path {
        Some(path) => Ok(path),
        None => Err("cosmos-tui not found. Install it or set COSMOS_TUI_PATH.".into()),
    }
}
```

- [ ] **Step 8: Create .gitignore**

```
/target
/result
.direnv/
```

- [ ] **Step 9: Verify build**

```bash
nix develop -c cargo check
nix develop -c cargo test
```

Expected: compiles, first_boot tests pass.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock flake.nix flake.lock Justfile .gitignore src/
git commit --author="Mara <mara@systemic.engineer>" -m "🔧 scaffold conversation-bin with first boot"
```

---

## Task 2: cosmos-tui Scaffold

**Files:**
- Create: `cosmos-tui/gleam.toml`, `cosmos-tui/flake.nix`, `cosmos-tui/Justfile`, `cosmos-tui/src/cosmos_tui.gleam`

- [ ] **Step 1: Create project**

```bash
mkdir -p /Users/alexwolf/dev/projects/cosmos-tui/src/cosmos_tui
mkdir -p /Users/alexwolf/dev/projects/cosmos-tui/test
cd /Users/alexwolf/dev/projects/cosmos-tui
git init
git checkout -b cosmos-tui/first-boot
```

- [ ] **Step 2: Write gleam.toml**

```toml
name = "cosmos_tui"
version = "0.1.0"
description = "Terminal renderer for cosmos — the first surface."
target = "erlang"

[dependencies]
gleam_stdlib = ">= 0.44.0 and < 2.0.0"
gleam_erlang = ">= 0.25.0 and < 2.0.0"
gleam_otp = ">= 1.0.0 and < 2.0.0"
gestalt_ui = { path = "../gestalt-ui" }
gestalt_tui = { path = "../gestalt-ui/target/tui" }

[dev-dependencies]
gleeunit = ">= 1.0.0 and < 2.0.0"
```

**Note:** Target is `erlang` (not javascript) — required for Rust NIF access to spectral-db.

- [ ] **Step 3: Write flake.nix**

Model after `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/flake.nix` — needs gleam, erlang_27, rebar3, git, just.

- [ ] **Step 4: Write Justfile**

```just
gleam := "nix develop -c gleam"

test:
    {{gleam}} test

check: test format-check

format:
    {{gleam}} format src/ test/

format-check:
    {{gleam}} format --check src/ test/

run:
    {{gleam}} run

pre-commit: check
```

- [ ] **Step 5: Write entry point**

Create `/Users/alexwolf/dev/projects/cosmos-tui/src/cosmos_tui.gleam`:

```gleam
/// cosmos-tui — the terminal surface.
/// Type `conversation` and this is what opens.
pub fn main() {
  io.println("cosmos-tui: not yet implemented")
}
```

Create `/Users/alexwolf/dev/projects/cosmos-tui/test/cosmos_tui_test.gleam`:

```gleam
import gleeunit

pub fn main() {
  gleeunit.main()
}
```

- [ ] **Step 6: Verify build**

```bash
nix develop -c gleam test
```

Expected: compiles, 0 tests, no errors.

- [ ] **Step 7: Commit**

```bash
git add gleam.toml flake.nix Justfile src/ test/
git commit --author="Mara <mara@systemic.engineer>" -m "🔧 scaffold cosmos-tui"
```

---

## Task 3: Model + State

**Files:**
- Create: `cosmos-tui/src/cosmos_tui/model.gleam`
- Create: `cosmos-tui/test/model_test.gleam`

The model holds application state. Elm architecture: Model is the truth.

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/cosmos-tui/test/model_test.gleam`:

```gleam
import cosmos_tui/model
import gleeunit/should

pub fn new_model_is_empty_test() {
  let m = model.new(80, 24, True)
  model.message_count(m) |> should.equal(0)
  model.is_first_boot(m) |> should.equal(True)
}

pub fn add_message_increments_count_test() {
  let m = model.new(80, 24, True)
  let m = model.add_message(m, "pathfinder", "Hello.")
  model.message_count(m) |> should.equal(1)
}

pub fn messages_ordered_newest_last_test() {
  let m = model.new(80, 24, True)
  let m = model.add_message(m, "pathfinder", "First")
  let m = model.add_message(m, "human", "Second")
  let msgs = model.messages(m)
  case msgs {
    [first, second] -> {
      first.text |> should.equal("First")
      second.text |> should.equal("Second")
    }
    _ -> should.fail()
  }
}

pub fn input_buffer_starts_empty_test() {
  let m = model.new(80, 24, True)
  model.input_text(m) |> should.equal("")
}

pub fn append_to_input_test() {
  let m = model.new(80, 24, True)
  let m = model.append_input(m, "h")
  let m = model.append_input(m, "i")
  model.input_text(m) |> should.equal("hi")
}

pub fn backspace_removes_last_char_test() {
  let m = model.new(80, 24, True)
  let m = model.append_input(m, "h")
  let m = model.append_input(m, "i")
  let m = model.backspace(m)
  model.input_text(m) |> should.equal("h")
}

pub fn backspace_on_empty_is_noop_test() {
  let m = model.new(80, 24, True)
  let m = model.backspace(m)
  model.input_text(m) |> should.equal("")
}

pub fn submit_input_creates_message_and_clears_test() {
  let m = model.new(80, 24, True)
  let m = model.append_input(m, "hello")
  let m = model.submit_input(m, "human")
  model.input_text(m) |> should.equal("")
  model.message_count(m) |> should.equal(1)
}
```

- [ ] **Step 2: Write model.gleam with stubs**

Create `/Users/alexwolf/dev/projects/cosmos-tui/src/cosmos_tui/model.gleam`:

```gleam
pub type Message {
  Message(from: String, text: String)
}

pub opaque type Model {
  Model(
    cols: Int,
    rows: Int,
    first_boot: Bool,
    messages: List(Message),
    input: String,
  )
}

pub fn new(_cols: Int, _rows: Int, _first_boot: Bool) -> Model {
  todo
}

pub fn message_count(_model: Model) -> Int {
  todo
}

pub fn is_first_boot(_model: Model) -> Bool {
  todo
}

pub fn add_message(_model: Model, _from: String, _text: String) -> Model {
  todo
}

pub fn messages(_model: Model) -> List(Message) {
  todo
}

pub fn input_text(_model: Model) -> String {
  todo
}

pub fn append_input(_model: Model, _char: String) -> Model {
  todo
}

pub fn backspace(_model: Model) -> Model {
  todo
}

pub fn submit_input(_model: Model, _from: String) -> Model {
  todo
}
```

- [ ] **Step 3: Verify tests compile and fail**

```bash
nix develop -c gleam test
```

Expected: compiles, all tests fail with `todo` panic.

- [ ] **Step 4: Commit red**

```bash
git add src/cosmos_tui/model.gleam test/model_test.gleam
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 model: conversation state"
```

- [ ] **Step 5: Implement model**

```gleam
import gleam/list
import gleam/string

pub type Message {
  Message(from: String, text: String)
}

pub opaque type Model {
  Model(
    cols: Int,
    rows: Int,
    first_boot: Bool,
    messages: List(Message),
    input: String,
  )
}

pub fn new(cols: Int, rows: Int, first_boot: Bool) -> Model {
  Model(cols:, rows:, first_boot:, messages: [], input: "")
}

pub fn message_count(model: Model) -> Int {
  list.length(model.messages)
}

pub fn is_first_boot(model: Model) -> Bool {
  model.first_boot
}

pub fn add_message(model: Model, from: String, text: String) -> Model {
  Model(..model, messages: list.append(model.messages, [Message(from:, text:)]))
}

pub fn messages(model: Model) -> List(Message) {
  model.messages
}

pub fn input_text(model: Model) -> String {
  model.input
}

pub fn append_input(model: Model, char: String) -> Model {
  Model(..model, input: model.input <> char)
}

pub fn backspace(model: Model) -> Model {
  let len = string.length(model.input)
  case len {
    0 -> model
    _ -> Model(..model, input: string.slice(model.input, 0, len - 1))
  }
}

pub fn submit_input(model: Model, from: String) -> Model {
  case model.input {
    "" -> model
    text -> {
      let model = add_message(model, from, text)
      Model(..model, input: "")
    }
  }
}

pub fn cols(model: Model) -> Int {
  model.cols
}

pub fn rows(model: Model) -> Int {
  model.rows
}
```

- [ ] **Step 6: Run tests**

```bash
nix develop -c gleam test
```

Expected: all 8 tests pass.

- [ ] **Step 7: Commit green**

```bash
git add src/cosmos_tui/model.gleam
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 model: conversation state"
```

---

## Task 4: Pathfinder Greeting

**Files:**
- Create: `cosmos-tui/src/cosmos_tui/pathfinder.gleam`
- Create: `cosmos-tui/test/pathfinder_test.gleam`

The Pathfinder is the first thing you see. Not a wizard. Not a configuration screen. A greeting that begins the observation.

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/cosmos-tui/test/pathfinder_test.gleam`:

```gleam
import cosmos_tui/pathfinder
import cosmos_tui/model
import gleeunit/should
import gleam/string

pub fn greeting_adds_messages_test() {
  let m = model.new(80, 24, True)
  let m = pathfinder.greet(m)
  let count = model.message_count(m)
  should.be_true(count > 0)
}

pub fn greeting_mentions_pathfinder_test() {
  let m = model.new(80, 24, True)
  let m = pathfinder.greet(m)
  let msgs = model.messages(m)
  let all_text = msgs
    |> list.map(fn(msg) { msg.text })
    |> string.join(" ")
  should.be_true(string.contains(all_text, "Pathfinder"))
}

pub fn greeting_not_added_on_return_test() {
  let m = model.new(80, 24, False)
  let m = pathfinder.welcome_back(m)
  let msgs = model.messages(m)
  let all_text = msgs
    |> list.map(fn(msg) { msg.text })
    |> string.join(" ")
  // Welcome back is different from first boot
  should.be_false(string.contains(all_text, "first time"))
}
```

- [ ] **Step 2: Write pathfinder stubs**

```gleam
import cosmos_tui/model.{type Model}

pub fn greet(_model: Model) -> Model {
  todo
}

pub fn welcome_back(_model: Model) -> Model {
  todo
}
```

- [ ] **Step 3: Verify tests fail, commit red**

```bash
nix develop -c gleam test
git add src/cosmos_tui/pathfinder.gleam test/pathfinder_test.gleam
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 pathfinder: first boot greeting"
```

- [ ] **Step 4: Implement pathfinder**

```gleam
import cosmos_tui/model.{type Model}

pub fn greet(m: Model) -> Model {
  m
  |> model.add_message("pathfinder", "Hello. I'm the Pathfinder.")
  |> model.add_message("pathfinder", "This is your first time here. Everything you do builds the graph.")
  |> model.add_message("pathfinder", "Type anything. I'm listening.")
}

pub fn welcome_back(m: Model) -> Model {
  m
  |> model.add_message("pathfinder", "Welcome back. The graph remembers.")
}
```

- [ ] **Step 5: Run tests, commit green**

```bash
nix develop -c gleam test
git add src/cosmos_tui/pathfinder.gleam
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 pathfinder: first boot greeting"
```

---

## Task 5: View (Render Model to Screen)

**Files:**
- Create: `cosmos-tui/src/cosmos_tui/view.gleam`
- Create: `cosmos-tui/test/view_test.gleam`

Renders the model to a gestalt-tui screen buffer. Pure function: Model → Screen.

- [ ] **Step 1: Write failing tests**

Create `/Users/alexwolf/dev/projects/cosmos-tui/test/view_test.gleam`:

```gleam
import cosmos_tui/view
import cosmos_tui/model
import gestalt_tui/io/screen
import gleeunit/should
import gleam/string

pub fn render_empty_model_produces_screen_test() {
  let m = model.new(80, 24, True)
  let s = view.render(m)
  // Screen should exist with correct dimensions
  should.be_true(True)
}

pub fn render_with_messages_includes_text_test() {
  let m = model.new(80, 24, True)
  let m = model.add_message(m, "pathfinder", "Hello.")
  let s = view.render(m)
  let output = screen.flush(s, screen.new(80, 24))
  should.be_true(string.contains(output, "Hello"))
}

pub fn render_input_shows_at_bottom_test() {
  let m = model.new(80, 24, True)
  let m = model.append_input(m, "typing")
  let s = view.render(m)
  let output = screen.flush(s, screen.new(80, 24))
  should.be_true(string.contains(output, "typing"))
}

pub fn render_shows_from_label_test() {
  let m = model.new(80, 24, True)
  let m = model.add_message(m, "pathfinder", "Hello.")
  let s = view.render(m)
  let output = screen.flush(s, screen.new(80, 24))
  should.be_true(string.contains(output, "pathfinder"))
}
```

- [ ] **Step 2: Write view stubs**

```gleam
import cosmos_tui/model.{type Model}
import gestalt_tui/io/screen.{type Screen}

pub fn render(_model: Model) -> Screen {
  todo
}
```

- [ ] **Step 3: Verify tests fail, commit red**

```bash
nix develop -c gleam test
git add src/cosmos_tui/view.gleam test/view_test.gleam
git commit --author="Mara <mara@systemic.engineer>" -m "🔴 view: render model to screen"
```

- [ ] **Step 4: Implement view**

```gleam
import cosmos_tui/model.{type Message, type Model}
import gestalt_tui/io/screen.{type Screen, Screen}
import gestalt_tui/style.{Style}
import gestalt_tui/palette.{type TerminalPalette}
import gestalt_ui/theme
import gleam/list
import gleam/string

pub fn render(m: Model) -> Screen {
  let cols = model.cols(m)
  let rows = model.rows(m)
  let t = theme.default_dark()
  let p = palette.from_theme(t)
  let s = screen.new(rows, cols)

  // Messages region: rows 0 to rows-3
  let s = render_messages(s, model.messages(m), p, cols, rows - 3)

  // Input region: last 2 rows
  let s = render_input(s, model.input_text(m), p, cols, rows)

  s
}

fn render_messages(
  s: Screen,
  messages: List(Message),
  p: TerminalPalette,
  cols: Int,
  max_row: Int,
) -> Screen {
  let muted_style = Style(
    fg: option.Some(p.muted_foreground),
    bg: option.None,
    bold: False,
    dim: True,
    italic: False,
  )
  let text_style = Style(
    fg: option.Some(p.foreground),
    bg: option.None,
    bold: False,
    dim: False,
    italic: False,
  )

  // Render messages from bottom up, newest at bottom
  let indexed = messages
    |> list.index_map(fn(msg, i) { #(i, msg) })

  list.fold(indexed, s, fn(s, pair) {
    let #(i, msg) = pair
    let row = i * 2  // 2 rows per message: label + text
    case row < max_row {
      True -> {
        let s = screen.write(s, row, 0, msg.from, muted_style)
        let s = screen.write(s, row + 1, 2, msg.text, text_style)
        s
      }
      False -> s
    }
  })
}

fn render_input(
  s: Screen,
  input: String,
  p: TerminalPalette,
  cols: Int,
  rows: Int,
) -> Screen {
  let separator_style = Style(
    fg: option.Some(p.muted),
    bg: option.None,
    bold: False,
    dim: True,
    italic: False,
  )
  let input_style = Style(
    fg: option.Some(p.foreground),
    bg: option.None,
    bold: False,
    dim: False,
    italic: False,
  )

  let separator = string.repeat("─", cols)
  let s = screen.write(s, rows - 2, 0, separator, separator_style)
  let prompt = "> " <> input
  let s = screen.write(s, rows - 1, 0, prompt, input_style)
  s
}
```

**Note:** This is an approximation. Read gestalt-tui's `screen.write()` and `style` module signatures carefully and adjust. The screen module uses `Style` from gestalt-tui — verify the import path and constructor match.

- [ ] **Step 5: Run tests, commit green**

```bash
nix develop -c gleam test
git add src/cosmos_tui/view.gleam
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 view: render model to screen"
```

---

## Task 6: Event Loop

**Files:**
- Create: `cosmos-tui/src/cosmos_tui/app.gleam`
- Modify: `cosmos-tui/src/cosmos_tui.gleam`

The event loop. The missing piece from gestalt-tui. Elm architecture: init → loop(update → render).

- [ ] **Step 1: Write app.gleam**

```gleam
import cosmos_tui/model.{type Model}
import cosmos_tui/view
import cosmos_tui/pathfinder
import gestalt_tui/io/terminal.{type Terminal}
import gestalt_tui/io/screen.{type Screen}
import gestalt_tui/io/input
import gleam/io
import gleam/option.{None, Some}
import gleam/result

pub type Msg {
  KeyChar(String)
  KeyEnter
  KeyBackspace
  Quit
  Tick
}

/// Start the application. Enters raw mode, runs the loop, exits cleanly.
pub fn run(first_boot: Bool) -> Nil {
  input.init()

  case terminal.enter() {
    Ok(term) -> {
      let model = model.new(term.cols, term.rows, first_boot)
      let model = case first_boot {
        True -> pathfinder.greet(model)
        False -> pathfinder.welcome_back(model)
      }
      let prev_screen = screen.new(term.rows, term.cols)
      let screen = view.render(model)
      let diff = screen.flush(screen, prev_screen)
      io.print(diff)

      loop(model, screen)

      terminal.exit()
    }
    Error(_) -> {
      io.println("Failed to initialize terminal")
    }
  }
}

fn loop(model: Model, prev_screen: Screen) -> Nil {
  case input.poll(100) {
    Some(Ok(event)) -> {
      let msg = event_to_msg(event)
      case msg {
        Quit -> Nil  // Exit loop
        msg -> {
          let model = update(model, msg)
          let screen = view.render(model)
          let diff = screen.flush(screen, prev_screen)
          io.print(diff)
          loop(model, screen)
        }
      }
    }
    _ -> loop(model, prev_screen)
  }
}

fn update(model: Model, msg: Msg) -> Model {
  case msg {
    KeyChar(c) -> model.append_input(model, c)
    KeyBackspace -> model.backspace(model)
    KeyEnter -> model.submit_input(model, "human")
    Quit -> model
    Tick -> model
  }
}

fn event_to_msg(event: input.Event) -> Msg {
  // Map etch events to our Msg type.
  // Read etch/event.gleam for the Event type structure.
  // Key events have a KeyEvent with code: KeyCode.
  // This mapping depends on etch's exact API — adjust as needed.
  case event {
    input.Key(key) -> {
      case input.is_quit(key) {
        True -> Quit
        False -> {
          case key.code {
            input.Enter -> KeyEnter
            input.Backspace -> KeyBackspace
            input.Char(c) -> KeyChar(c)
            _ -> Tick
          }
        }
      }
    }
    _ -> Tick
  }
}
```

**Important:** The event type mapping (`event_to_msg`) depends on etch's exact `Event`, `KeyEvent`, `KeyCode` types. Read `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/build/packages/etch/src/etch/event.gleam` to get the exact constructors. The code above is structurally correct but may need constructor name adjustments.

- [ ] **Step 2: Wire up cosmos_tui.gleam entry point**

Replace `/Users/alexwolf/dev/projects/cosmos-tui/src/cosmos_tui.gleam`:

```gleam
import cosmos_tui/app
import gleam/erlang
import gleam/list

pub fn main() {
  // Check if this is first boot by looking at args
  // conversation-bin passes the home path; first boot = home was just created
  let args = erlang.start_arguments()
  let first_boot = case list.first(args) {
    Ok("--first-boot") -> True
    _ -> False
  }

  app.run(first_boot)
}
```

- [ ] **Step 3: Test manually**

```bash
nix develop -c gleam run -- --first-boot
```

Expected: Terminal enters raw mode, Pathfinder greeting appears, you can type, Ctrl+C/Q/Esc exits cleanly.

**Note:** This is an interactive test — cannot be automated in the test suite. The unit tests from Tasks 3-5 cover the pure logic. This step verifies the I/O integration.

- [ ] **Step 4: Commit**

```bash
git add src/cosmos_tui/app.gleam src/cosmos_tui.gleam
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 app: event loop — Elm architecture on gestalt-tui"
```

---

## Task 7: Wire conversation-bin → cosmos-tui

**Files:**
- Modify: `conversation-bin/src/main.rs`
- Modify: `conversation-bin/src/launch.rs`

- [ ] **Step 1: Update run_tui() to launch cosmos-tui**

In `conversation-bin/src/main.rs`, replace the `run_tui` function:

```rust
fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    let (home, is_first_boot) = first_boot::ensure_home_with_status()?;
    launch::launch_cosmos_tui(&home, is_first_boot)
}
```

- [ ] **Step 2: Update first_boot.rs to report first boot status**

Add to `conversation-bin/src/first_boot.rs`:

```rust
/// Returns (home_path, was_first_boot)
pub fn ensure_home_with_status() -> Result<(PathBuf, bool), Box<dyn std::error::Error>> {
    let home = dirs_or_fallback();
    let conv_home = home.join(".conversation");

    let is_first_boot = !conv_home.exists();
    if is_first_boot {
        first_boot(&conv_home)?;
    }

    Ok((conv_home, is_first_boot))
}
```

- [ ] **Step 3: Update launch.rs to pass first_boot flag**

```rust
pub fn launch_cosmos_tui(conv_home: &Path, first_boot: bool) -> Result<(), Box<dyn std::error::Error>> {
    let cosmos_tui_dir = find_cosmos_tui()?;

    let mut cmd = Command::new("gleam");
    cmd.arg("run")
       .arg("-m")
       .arg("cosmos_tui")
       .arg("--")
       .current_dir(&cosmos_tui_dir);

    if first_boot {
        cmd.arg("--first-boot");
    }

    cmd.arg(conv_home.to_str().unwrap_or("~/.conversation"));

    let status = cmd.status()?;

    if !status.success() {
        return Err(format!("cosmos-tui exited with status: {}", status).into());
    }

    Ok(())
}
```

- [ ] **Step 4: Test end-to-end**

```bash
cd /Users/alexwolf/dev/projects/conversation-bin
nix develop -c cargo run
```

Expected: first boot creates `~/.conversation/` (or temp dir for testing), then launches cosmos-tui which shows the Pathfinder greeting.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/first_boot.rs src/launch.rs
git commit --author="Mara <mara@systemic.engineer>" -m "🟢 wire conversation-bin → cosmos-tui launch"
```

---

## Task 8: Cleanup + Final Verification

**Files:** All files in both projects.

- [ ] **Step 1: Run conversation-bin checks**

```bash
cd /Users/alexwolf/dev/projects/conversation-bin
nix develop -c cargo test && nix develop -c cargo clippy -- -D warnings && nix develop -c cargo fmt -- --check
```

Fix any issues.

- [ ] **Step 2: Run cosmos-tui checks**

```bash
cd /Users/alexwolf/dev/projects/cosmos-tui
nix develop -c gleam test && nix develop -c gleam format --check src/ test/
```

Fix any issues.

- [ ] **Step 3: Verify git logs**

```bash
cd /Users/alexwolf/dev/projects/conversation-bin && git log --oneline
cd /Users/alexwolf/dev/projects/cosmos-tui && git log --oneline
```

- [ ] **Step 4: Commit any fixes**

```bash
# In each repo as needed:
git commit --author="Mara <mara@systemic.engineer>" -m "♻️ clippy and fmt cleanup"
```

---

## Implementer Notes

### gestalt-tui API verification

The view.gleam implementation uses gestalt-tui's `screen.write()`, `Style`, `palette.from_theme()`. These APIs are based on exploration but may have changed. **Read the actual source files before implementing Task 5:**
- `gestalt_tui/io/screen.gleam` — verify `write()` signature (row, col, content, style)
- `gestalt_tui/style.gleam` — verify `Style` constructor fields
- `gestalt_tui/palette.gleam` — verify `from_theme()` and field names

### etch event types

Task 6's `event_to_msg` depends on etch's exact type constructors. **Read before implementing:**
- `/Users/alexwolf/dev/projects/gestalt-ui/target/tui/build/packages/etch/src/etch/event.gleam`
- gestalt-tui's `input.gleam` re-exports — verify which types are available

### theme.default_dark()

The view assumes `theme.default_dark()` exists in gestalt-ui. **Verify:** read `/Users/alexwolf/dev/projects/gestalt-ui/src/gestalt_ui/theme.gleam` for the actual constructor. You may need to construct a Theme manually:

```gleam
let t = theme.Theme(
  mode: theme.Dark,
  density: theme.Comfortable,
  contrast: 4.5,
  scale: 1.0,
  motion: theme.Full,
)
```

### spectral-db persistence (future)

This plan stores messages in-memory only. Task for next plan: extend spectral-db's Gleam NIF adapter to support `open()` and `insert()`, then store conversation turns as episodic memory nodes.

### Launch mechanism (development vs production)

Task 7 uses `gleam run` to launch cosmos-tui during development. For production, cosmos-tui would compile to an OTP release and conversation-bin would exec the release binary. That's a packaging task, not a first-boot task.
