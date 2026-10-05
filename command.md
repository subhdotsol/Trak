# trak — all the commands, what each one does, and the args to pass

**Pattern for every usage block below:** `when → run → what you'll see → args/flags`.

Run on the project you want to analyze (any directory inside it):

```sh
cd /path/to/project
trak <command> <args...>
```

---

## Quick reference — all commands

| Command | What it answers | Try it on |
| --- | --- | --- |
| `search` | Which lines mention this text? | `trak search discover` |
| `symbol` | Where is this symbol declared? | `trak symbol discover` |
| `fn` | Show a function: definition + what it calls | `trak fn discover` |
| `calls` | What does this function call? (`--tree` for forward tree) | `trak calls discover` |
| `calls --tree` | Full callees hierarchy as a tree | `trak calls discover --tree` |
| `callers` | Which functions call it? (`--tree` for reverse tree) | `trak callers from_dir` |
| `callers --tree` | Full callers hierarchy as a tree | `trak callers from_dir --tree` |
| `path` | Is there a call chain A→…→B? Or the route to a log message? | `trak path main discover` |
| `refs` | Every reference (definition + call sites) to a symbol | `trak refs format_number` |
| `impact` | If I change this fn, what breaks? (transitive callers) | `trak impact discover` |
| `relevant` | Containing fn + callers/callees that touch this text | `trak relevant from_dir` |
| `architecture` | Repo overview: file tree + symbol/function counts | `trak architecture` |
| `history` | Git history of a symbol/function | `trak history from_dir` |
| `blame` | Who changed a `file:line`? | `trak blame crates/trak-core/src/repo.rs:20` |
| `diff` | Uncommitted working-tree changes | `trak diff` |
| `open` | Open a symbol's location in your editor | `trak open discover` |

Formats available on data commands (`search`, `symbol`, `fn`, `calls`, `callers`, `path`,
`refs`, `impact`, and `relevant`): `--format terminal` (default), `json`, `markdown`,
`mermaid`, `dot`. See [Formats](#formats) at the end.

---

## 1. search — find lines containing text

```sh
trak search discover
trak search "current_dir"
```

What you'll get: every matching `file:line: text`, each hit tagged `[hit]`:

```
search `discover`
53 hit(s)

./README.md
1: Trak is a local CLI tool ... unfamiliar Rust codebases ...  [hit]

./crates/trak-core/src/repo.rs
13: pub fn discover() -> crate::Result<Self> {  [hit]
20: pub fn from_dir(dir: &Path) -> crate::Result<Self> {  [hit]
```

Args / flags:

- `<query>` — the text to search for (required).
- `--format <terminal|json|markdown|mermaid|dot>` — output format (default `terminal`).

> Search is plain text: it matches comments, docs, and strings too. It is **not**
> limited to code, so a std call like `current_dir` still appears here — use the
> graph commands (`fn`, `calls`, `callers`, `impact`) when you want **codebase
> functions only**.

## 2. symbol — show a symbol declaration

```sh
trak symbol discover
trak symbol Function
```

What you'll get: the declaration with `file:line` and a `[symbol]` tag:

```
symbol `discover`

./crates/trak-core/src/repo.rs
13: pub fn discover() -> crate::Result<Self> {  [symbol]
```

Args / flags: `<name>` (required) · `--format`.

## 3. fn — show a function: definition + what it calls

```sh
trak fn discover
```

What you'll get: where it's defined plus the call edges **to functions this
codebase itself defines** — std/library calls like `unwrap` or `read_line` are
pruned, so you only ever see the team's own code:

```
fn `discover`

./crates/trak-core/src/repo.rs
13: pub fn discover() -> crate::Result<Self> {  [definition]

discover → from_dir
```

A call is kept when it resolves into this repository, so `crate::ui::render(..)`
and `self.render(..)` are edges while `input.trim(..)` is not — even if this
codebase defines its own `trim`. A name imported from another crate in the
workspace is an edge, including under its alias (`use sibling::paint as draw`
records `paint`), and one imported from outside is not.

Args / flags: `<name>` (required) · `--format`.

## 4. calls — what a function directly calls

```sh
trak calls discover
trak calls parse_raw
```

What you'll get: only repo-defined callees:

```
discover → from_dir
```

Args / flags: `<name>` (required) · `--format` · `--tree`.

A leaf function (nothing repo-defined to call) shows `0 calls.`

With `--tree`, the whole forward hierarchy instead of one level — the mirror of
`callers --tree`:

```
discover
└─ from_dir
   └─ display_path
```

## 5. callers — who calls this function

```sh
trak callers from_dir
```

What you'll get:

```
callers from_dir — 1 function
discover
```

Args / flags: `<name>` (required) · `--format` · `--tree`.

## 6. callers --tree — full reverse call tree

```sh
trak callers from_dir --tree
```

What you'll get: every path back to `main`, as a tree:

```
from_dir
└─ discover
   ├─ run
   │  ├─ main
   │  └─ search
   └─ run_context
```

Args / flags: `<name>` (required) · `--tree` (render a tree instead of a flat
list) · `--format`.

## 7. path — is there a call chain A → … → B?

```sh
trak path main discover
trak path main "failed to read input: {error}"
```

What you'll get: the shortest repo-defined call path (or `no path`):

```
main → run → discover
```

Either endpoint may be a function name **or** a message you saw in the logs. A
message resolves to the function that emits it, so you get the execution path
to that line:

```
path from `main` to `failed to read input: {error}`
2 hop(s)

./src/main.rs
29: eprintln!("failed to read input: {error}");  [message]

main → deploy → confirm
```

A `{name}` placeholder you type is matched loosely, so the query above also
finds a call site that interpolates positionally
(`"failed to read input: {}"`). When a message is emitted from several
functions, every reachable route is listed, shortest first.

Args / flags: `<from>` and `<to>` — function names or messages (required) ·
order matters (direction of calls) · `--format`.

## 8. refs — every reference to a symbol

```sh
trak refs format_number
```

What you'll get: the declaration plus each call site:

```
references to `format_number`
  1 definition, 2 call sites, 3 references
```

Args / flags: `<name>` (required) · `--format`.

## 9. impact — if I change this function, what breaks?

```sh
trak impact discover
```

What you'll get: every function transitively affected (callers of callers, up
to and including `main`), excluding the function itself:

```
impact of `discover` — 1 function
main
```

Args / flags: `<name>` (required) · `--format`.

## 10. relevant — containing function and its context for some text

```sh
trak relevant from_dir
```

What you'll get: the function that contains the match plus the callers/callees
that give it context:

```
relevant to `from_dir`

from_dir
  definition: ./crates/trak-core/src/repo.rs:20
  caller: discover
```

Args / flags: `<query>` (required) · `--format`.

## 11. architecture — repository overview

```sh
trak architecture
```

What you'll get: the file tree with symbol/function counts:

```
trak
71 source files · 270 symbols · 149 functions
├── crates/
│   ├── trak-core/
│   └── ...
```

Args / flags: **(none)** — terminal only.

## 12. history — git history of a symbol

```sh
trak history from_dir
```

What you'll get: the commits that touched the function's lines:

```
from_dir
  d7b6827 initial demo project (2 changes)
```

Args / flags: `<name>` (required) — terminal only.

## 13. blame — who changed a location

```sh
trak blame crates/trak-core/src/repo.rs:20
```

What you'll get: commit, author, date, and the content of that line.

Args / flags: `<loc>` as `path:line` (required) — terminal only.

## 14. diff — uncommitted working-tree changes

```sh
trak diff
```

What you'll get: the current uncommitted diff.

Args / flags: **(none)** — terminal only.

## 15. open — jump to a symbol in your editor

```sh
trak open discover
```

What you'll get: opens the editor (from `TRAK_EDITOR`, else `EDITOR`, else a
discovered editor: VS Code, Zed, Neovim, Vim) at the symbol's line.

Args: `<name>` (required).

---

## Formats

Add `--format <value>` to data commands (`search`, `symbol`, `fn`, `calls`,
`callers`, `path`, `refs`, `impact`, `relevant`):

| Value | Output |
| --- | --- |
| `terminal` | Colored text (default) |
| `json` | Machine-readable JSON |
| `markdown` | Markdown document |
| `mermaid` | A `graph TD` flowchart |
| `dot` | Graphviz DOT |

Example:

```sh
trak calls discover --format json
trak callers from_dir --format mermaid
```
