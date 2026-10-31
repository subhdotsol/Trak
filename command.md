# trak — all the commands, what each one does, and the args to pass

**Pattern for every usage block below:** `when → run → what you'll see → args/flags`.

Run on the project you want to analyze (any directory inside it):

```sh
cd /path/to/project
trak <command> <args...>
```

Every example below was run against the trak repository itself, so you can
clone it and try them as written. Output is trimmed where marked `...`, and
counts will drift as the code changes.

---

## Quick reference — all commands

| Command | What it answers | Try it on |
| --- | --- | --- |
| `search` | Which lines mention this text? | `trak search discover` |
| `symbol` | Where is this symbol declared? | `trak symbol discover` |
| `fn` | Show a function: definition + what it calls | `trak fn discover` |
| `calls` | What does this function call? | `trak calls in_dir` |
| `calls --tree` | Full callees hierarchy as a tree | `trak calls discover --tree` |
| `callers` | Which functions call it? | `trak callers from_dir` |
| `callers --tree` | Full callers hierarchy as a tree | `trak callers from_dir --tree` |
| `path` | Is there a call chain A→…→B? Or the route to a message? | `trak path main from_dir` |
| `refs` | Every reference (definition + call sites) to a symbol | `trak refs from_dir` |
| `impact` | If I change this fn, what breaks? (transitive callers) | `trak impact discover` |
| `relevant` | Containing fn + callers/callees for each text match | `trak relevant from_dir` |
| `log` / `error` / `panic` | Where does this message come from? | `trak error "no diff"` |
| `architecture` | Repo overview: file tree + symbol/function counts | `trak architecture` |
| `history` | Git history of a symbol/function | `trak history from_dir` |
| `blame` | Who changed a `file:line`? | `trak blame crates/trak-core/src/repo.rs:20` |
| `diff` | Uncommitted working-tree changes | `trak diff` |
| `open` | Open a symbol or file location in your editor | `trak open discover` |
| *(no command)* / `interactive` | Open an interactive session | `trak` |

`--format` works on these commands only: `search`, `symbol`, `fn`, `calls`,
`callers`, `path`, `refs`, `impact`. Values: `terminal` (default), `json`,
`markdown`, `mermaid`, `dot`. See [Formats](#formats) at the end. Every other
command prints terminal text and rejects `--format`.

Global flags: `trak --help`, `trak --version`, and `trak <command> --help`.

Exit codes: `0` on success (including "no call path" and "0 call(s)"), `1`
when nothing matches (`error: nothing found for ...`), `2` on bad arguments.

---

## 1. search — find lines containing text

```sh
trak search discover
trak search "current_dir"
```

What you'll get: a count, then every match grouped by file as `line: text`,
each tagged `[hit]`:

```
search `current_dir`
10 hit(s)
...

./crates/trak-core/src/repo.rs
14: let start = std::env::current_dir()?;  [hit]

./crates/trak/src/commands/open.rs
67: if let Ok(cwd) = std::env::current_dir() {  [hit]
...
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
trak symbol Repository
```

What you'll get: each declaration with its line and kind tag (`[fn]`,
`[struct]`, `[impl]`, `[enum]`, ...):

```
symbol `Repository`

./crates/trak-core/src/repo.rs
6: struct Repository  [struct]
10: impl Repository  [impl]
```

Args / flags: `<name>` (required) · `--format`.

## 3. fn — show a function: definition + what it calls

```sh
trak fn discover
```

What you'll get: where it's defined plus the call edges **to functions this
codebase itself defines** — std/library calls like `unwrap` or `current_dir`
are pruned, so you only ever see the team's own code:

```
fn `discover`

./crates/trak-core/src/repo.rs
13: pub fn discover() -> crate::Result<Self> {  [definition]

nodes:
  pub fn discover() -> crate::Result<Self> {

discover → from_dir
```

A call is kept when it resolves into this repository, so `crate::ui::render(..)`,
`Self::from_dir(..)`, and `self.render(..)` are edges while `input.trim(..)` is
not — even if this codebase defines its own `trim`. A name imported from
another crate in the workspace is an edge, including under its alias
(`use sibling::paint as draw` records `paint`), and one imported from outside
is not.

Args / flags: `<name>` (required) · `--format`.

## 4. calls — what a function directly calls

```sh
trak calls in_dir
trak calls discover --tree
```

What you'll get: only repo-defined callees, one level deep:

```
calls of `in_dir`
2 call(s)

./crates/trak/src/interactive/session.rs
30: pub fn in_dir(dir: &std::path::Path) -> Result<Session> {  [definition]

nodes:
  pub fn in_dir(dir: &std::path::Path) -> Result<Session> {
  from_repo
  from_dir

in_dir → from_repo
in_dir → from_dir
```

A leaf function (nothing repo-defined to call) shows `0 call(s)`.

With `--tree`, the whole forward hierarchy instead of one level — the mirror of
`callers --tree`:

```
discover
└─ from_dir
```

Args / flags: `<name>` (required) · `--tree` · `--format`.

> When several functions share a name (every command module has a `run`),
> `calls` picks one of them. Use `trak symbol <name>` to see them all, and
> prefer a unique name.

## 5. callers — who calls this function

```sh
trak callers from_dir
```

What you'll get:

```
callers of `from_dir`
3 caller(s)

./crates/trak-core/src/repo.rs
13: pub fn discover() -> crate::Result<Self> {  [caller]
...

in_dir → from_dir
discover → from_dir
fixture_trees_are_not_searched → from_dir
```

A function nobody calls prints `no callers of `<name>``.

Args / flags: `<name>` (required) · `--tree` · `--format`.

## 6. callers --tree — full reverse call tree

```sh
trak callers from_dir --tree
```

What you'll get: every path back up to the entry points, as a tree:

```
from_dir
├─ in_dir
│  ├─ session
│  │  ├─ the_query_screen_names_the_repository_and_file_count
│  ...
├─ discover
│  ├─ run
│  │  └─ main
...
```

Args / flags: `<name>` (required) · `--tree` (render a tree instead of a flat
list) · `--format`.

## 7. path — is there a call chain A → … → B?

```sh
trak path main from_dir
trak path main "no callers of {name}"
```

What you'll get: the shortest repo-defined call path:

```
path from `main` to `from_dir`
3 hop(s)

main → run → discover → from_dir
```

If there is none you get `no call path from `A` to `B`` (exit code `0`).
Order matters: `trak path from_dir main` has no path, because calls go
`main → … → from_dir`.

Either endpoint may be a function name **or** a message you saw in the output
or logs. A message resolves to the function that emits it, so you get the
execution path to that line:

```
path from `main` to `no callers of {name}`
1 hop(s)

./crates/trak/src/commands/callers.rs
17: println!("no callers of `{name}`");  [message]

main → run
```

A `{name}` placeholder you type is matched loosely, so it also finds a call
site that interpolates something else (`{}`, `{fn}`, ...). When a message is
emitted from several functions, every reachable route is listed, shortest
first. A message that only appears outside any function (docs, a `const`)
is an error: `error: `...` is at FILE:LINE, outside any function`.

Args / flags: `<from>` and `<to>` — function names or messages (required) ·
`--format`.

## 8. refs — every reference to a symbol

```sh
trak refs from_dir
```

What you'll get: the declaration plus every function that calls it:

```
references to `from_dir`
5 reference(s)

./crates/trak-core/src/repo.rs
20: fn declaration  [definition]
20: pub fn from_dir(dir: &Path) -> crate::Result<Self> {  [definition]
13: pub fn discover() -> crate::Result<Self> {  [caller]
...
```

Args / flags: `<name>` (required) · `--format`.

## 9. impact — if I change this function, what breaks?

```sh
trak impact discover
```

What you'll get: every function transitively affected (callers of callers, up
to and including `main`), excluding the function itself, grouped by file:

```
impact of changing `discover`
74 affected function(s)

./crates/trak-editor/src/lib.rs
72: pub fn detect() -> Option<Editor> {  [affected]
...
```

Args / flags: `<name>` (required) · `--format`.

## 10. relevant — containing function and its context for some text

```sh
trak relevant from_dir
```

What you'll get: every text match; for matches inside a function, that
function's location, signature, callers, and callees:

```
relevant to: `from_dir`

./crates/trak-core/src/repo.rs:13
  pub fn discover() -> crate::Result<Self> {
  called from: run, run, run, ..., run_context, new
  calls: from_dir
  match: ./crates/trak-core/src/repo.rs:15: Self::from_dir(&start)
...
```

Matches outside any function (docs, comments at file level) are printed as
plain `file:line: text`.

Args / flags: `<query>` (required) — terminal only, **no `--format`**.

## 11. log / error / panic — where does this message come from?

```sh
trak log "no callers of"
trak error "no diff"
trak panic "nothing found"
```

What you'll get: the same output as `relevant`, under a heading that names
what you were looking for:

```
error message: `no diff`

./crates/trak/src/commands/diff.rs:4
  pub fn run() -> Result<(), Error> {
  called from: run, main, search, search_message
  calls: discover, diff, root
  match: ./crates/trak/src/commands/diff.rs:8: println!("no diff (clean working tree)");
```

The three commands behave identically; pick whichever reads best. To get the
call chain from `main` to the message, use `trak path main "<message>"`.

Args / flags: `<message>` (required) — terminal only.

## 12. architecture — repository overview

```sh
trak architecture
```

What you'll get: the file tree with symbol/function counts:

```
trak
68 source files · 446 symbols · 319 functions

├── .githooks/ (0 files)
├── .github/ (4 files)
│   ├── ISSUE_TEMPLATE/ (2 files)
...
```

Args / flags: **(none)** — terminal only.

## 13. history — git history of a symbol

```sh
trak history from_dir
```

What you'll get: one line per commit that touched the function's lines,
newest first:

```
b8ec7a5 feat: add --tree to calls, mirroring callers --tree (#2)
b15a7f4 feat: add interactive session state with a lazily parsed graph
...
```

Args / flags: `<name>` (required) — terminal only. Needs a git repository.

## 14. blame — who changed a location

```sh
trak blame crates/trak-core/src/repo.rs:20
trak blame crates/trak-core/src/repo.rs
```

What you'll get: `git blame` output — commit, author, date, line number, and
content:

```
^fe2f60c (subh 2026-09-25 00:52:17 +0530 20)     pub fn from_dir(dir: &Path) -> crate::Result<Self> {
```

Args / flags: `<loc>` as `path:line` for one line, or just `path` for the
whole file (required) — terminal only. Needs a git repository.

## 15. diff — uncommitted working-tree changes

```sh
trak diff
```

What you'll get: the current uncommitted diff, or
`no diff (clean working tree)`.

Args / flags: **(none)** — terminal only. Needs a git repository.

## 16. open — jump to a symbol or file in your editor

```sh
trak open discover
trak open crates/trak/src/main.rs:25   # file at a line
trak open crates/trak/src/main.rs:25:4 # file at a line and column
```

What you'll get: resolves the argument to a location, opens the editor, and
prints e.g. `opened ./crates/trak/src/main.rs:25 in Zed`. A symbol or function
name (say, `discover`) resolves to its definition; a path opens the file
itself, jumping to `line` (or `line:col`) when given. The editor is chosen
from, in order:

1. `TRAK_EDITOR` — an explicit override, possibly with arguments (`zed --wait`).
2. The editor trak is running inside — `$ZED_TERM`, `$NVIM`, `$TERM_PROGRAM`,
   or the parent process tree.
3. `$VISUAL`, then `$EDITOR`.
4. An installed CLI: Zed, VS Code / Cursor, Neovim, Vim, Helix.

So from a Zed terminal, `trak open` reuses the open Zed window instead of
spawning VS Code or a new editor.

Args: `<name>` (required) — a symbol, function, or `path[:line[:col]]`.

---

## Interactive mode

```sh
trak
trak interactive
```

Three screens, one prompt each.

**Query.** Type a function name to open it directly, or any other text to search
for it. Resolution order is the same as `trak path`: a name the repository
defines wins, and anything else is treated as a message. A match that sits
outside every function — a `const`, a doc comment — reports where it is, because
there is no call path leading to it.

**Matches.** Numbered `[n] file:line  function()`, capped at 20; anything past
that is reported as not selectable. The count is of *functions* — a message
emitted twice in one function is one entry — and occurrences outside every
function are counted on their own line, because nothing calls them. A name with
more than one definition lands here too, rather than silently picking one.
Enter a number to open it, `b` to go back, `q` to quit.

**Detail.** The function's span, one level of callers, one level of callees, then:

| Key | Shows |
| --- | --- |
| `1` | The function's source, numbered, capped at 60 lines |
| `2` | Its immediate callers |
| `3` | Its immediate callees |
| `4` | The whole tree of callers, out to the entry point |
| `5` | The whole tree of callees |
| `6` | Opens the location in your editor |
| `b` | Back to the match list, or to the query if there was none |
| `q` | Quit |

An action prints its output and leaves you on the detail screen, so you can run
another without navigating back.

A trace is capped at 200 lines, as `[1]` is capped at 60.

Bare `trak` opens a session only when stdin is a terminal. Off a terminal it
prints usage and exits with code `2`, so a script or CI step that runs `trak`
with no arguments fails fast rather than waiting on a prompt; `trak interactive`
is the explicit entry point for a piped script (`echo q | trak interactive`).

Interactive mode takes no flags and renders terminal text only; use the
subcommands with `--format` for machine-readable output.

## Formats

Add `--format <value>` to `search`, `symbol`, `fn`, `calls`, `callers`, `path`,
`refs`, or `impact`:

| Value | Output |
| --- | --- |
| `terminal` | Colored text (default) |
| `json` | Machine-readable JSON (`title`, `summary`, `hits`, `nodes`, `edges`) |
| `markdown` | Markdown document |
| `mermaid` | A `graph TD` flowchart in a ```` ```mermaid ```` block |
| `dot` | Graphviz DOT (`digraph trak { ... }`) |

Example:

```sh
trak calls in_dir --format json
trak callers from_dir --format mermaid
trak fn discover --format dot | dot -Tsvg > discover.svg   # needs Graphviz
```
