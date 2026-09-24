# Trak

Trak is a local CLI tool for navigating and understanding unfamiliar Rust codebases. It helps you find symbols, follow callers and callees, trace code paths, and discover contextually related code. Instead of reading thousands of files or relying on AI, Trak shows you where to look and how the code connects.

Trak: find it. Follow it. Understand it.

## Features

- Text search across the whole repository
- Symbol lookup for structs, enums, traits, impls, and functions
- Caller and callee tracking for any function
- Shortest path between two functions in the call graph
- Impact analysis: what breaks if you change a function
- Reference lookup for any symbol
- Repository architecture overview
- Built-in git integration: history, blame, and diff
- Output as text, JSON, Markdown, Mermaid, or Graphviz DOT
- Opens symbol locations directly in your editor
- Zero dependencies in the parser, fast on large codebases

## Setup

Requirements:

- Rust stable toolchain. Install via [rustup](https://rustup.rs/) if you do not have it.

Steps:

1. Clone the repository.

```sh
git clone https://github.com/subhdotsol/trak.git
cd trak
```

2. Build the release binary.

```sh
cargo build --release
```

3. Install the `trak` binary on your PATH.

```sh
cargo install --path crates/trak
```

4. Verify the install.

```sh
trak --version
```

To build without installing, use `target/release/trak` directly.

## Quick start

Run Trak from inside any Rust codebase. It discovers the repository automatically by walking up to the nearest `.git` directory.

```sh
trak architecture           # overview of structs, functions, and crates
trak fn main                # signature, parameters, and calls of main
trak callers parse --tree   # reverse call tree up to main
trak calls parse            # functions that parse calls
trak path start_fn end_fn   # shortest call path between two functions
trak impact validate        # everything affected by changing validate
trak refs Point             # every reference to Point
trak search "TODO"          # repository-wide text search
trak open main              # open main in your editor
```

## Command reference

| Command | Description |
| --- | --- |
| `search <query>` | Find every line containing the query |
| `symbol <name>` | Show the declaration of a symbol |
| `fn <name>` | Show a function: signature, parameters, and calls |
| `callers <name>` | Find functions that call this function |
| `callers <name> --tree` | Render the reverse call tree instead |
| `calls <name>` | Find the functions called by this function |
| `path <from> <to>` | Find the shortest call path between two functions |
| `refs <name>` | Find every reference to a symbol |
| `impact <name>` | Analyze what changing a function would affect |
| `architecture` | Show the repository architecture |
| `history <name>` | Show git history for a symbol |
| `blame <loc>` | Show blame for a file or a `file:line` location |
| `diff` | Show the working tree diff |
| `relevant <query>` | Jump to the function context around a query |
| `log <message>` | Find where a log message originates |
| `error <message>` | Find where an error message originates |
| `panic <message>` | Find where a panic message originates |
| `open <name>` | Open a symbol or function in your editor |

### Output formats

The data commands accept `--format`, which supports `terminal` (default), `json`, `markdown`, `mermaid`, and `dot`.

```sh
trak callers parse --format mermaid     # call graph for diagrams
trak fn main --format json              # structured output for scripts
```

The `architecture`, `history`, `blame`, `diff`, `relevant`, `log`, `error`, `panic`, and `open` commands render in the terminal.

### Editing

`trak open <name>` resolves the symbol to a `file:line` location and launches your editor. The editor is chosen from, in order:

1. The `TRAK_EDITOR` environment variable.
2. The `EDITOR` environment variable.
3. A detected editor: VS Code, Zed, Neovim, or Vim.

## Project layout

This is a Cargo workspace.

| Crate | Responsibility |
| --- | --- |
| `crates/trak` | The CLI binary: clap definitions and command dispatch |
| `crates/trak-core` | Repository discovery, file traversal, and the symbol model |
| `crates/trak-rust` | A dependency-free heuristic Rust parser |
| `crates/trak-analysis` | Callers, callees, path finding, impact, and references |
| `crates/trak-search` | Repository-wide text search |
| `crates/trak-git` | Git history, blame, and diff integrations |
| `crates/trak-output` | Report model and terminal, JSON, Markdown, Mermaid, and DOT renderers |
| `crates/trak-editor` | Editor detection and file:line opening |

A design rule holds throughout: analysis logic lives in the crates, and clap stays only at the CLI boundary in `crates/trak`.

## Development

```sh
cargo build --workspace
cargo test --workspace --all-features
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development workflow, style rules, and the checklist to run before you push.

## License

MIT