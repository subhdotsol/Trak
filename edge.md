# trak — edge cases, end to end

What trak does at the edges: other languages, odd repos, odd files, odd Rust,
and odd input to each command. Everything below was checked by running trak
v0.1.0 against throwaway test repos, not inferred from the docs.

Legend: ✅ works · ⚠️ works with a catch · ❌ doesn't work

---

## 1. The one-line summary

trak has **two engines**:

| Engine | Used by | Languages |
| --- | --- | --- |
| **Text search** (ripgrep engine) | `search`, `relevant`, `log`, `error`, `panic`, the message half of `path` | Any text file with a known extension (Rust, Python, JS/TS, Go, Java, C/C++, Ruby, Markdown, TOML, YAML, ...) |
| **Rust parser** (heuristic, line based) | `symbol`, `fn`, `calls`, `callers`, `path`, `refs`, `impact`, `open <name>`, the counts in `architecture`, interactive mode | **`.rs` files only** |
| **git** (shells out to `git`) | `history`, `blame`, `diff` | Any language — git doesn't care |

So in a non-Rust repo you get text search and git, and nothing that needs to
understand code.

---

## 2. Running trak in a non-Rust repo (Python, JS/TS, Go, ...)

Tested on a small Python package (`main()` → `run()` → `helper()`, a class with
methods, a `raise ValueError("...")`).

| Command | Result in a Python repo |
| --- | --- |
| `search <text>` | ✅ Full support, all `--format`s |
| `relevant <text>` | ⚠️ Shows the matching lines, but never the containing function, callers or callees (there are no functions as far as trak knows) |
| `log` / `error` / `panic <msg>` | ⚠️ Same as `relevant`: finds the line, no function context |
| `architecture` | ⚠️ File tree is correct, but always `0 symbols · 0 functions` |
| `history <name>` | ✅ (it's `git log -S`, language-agnostic) |
| `blame <file:line>` | ✅ |
| `diff` | ✅ |
| `symbol <name>` | ❌ `error: nothing found for symbol` (exit 1) |
| `fn <name>` / `calls <name>` | ❌ `error: nothing found for function` (exit 1) |
| `refs <name>` | ❌ `error: nothing found for references` (exit 1) |
| `open <name>` | ❌ `nothing found` — but `open path/to/file.py:12` ✅ works |
| `callers <name>` | ⚠️ **Misleading**: prints `no callers of `helper`` and exits **0**, even though `run()` calls it |
| `impact <name>` | ⚠️ **Misleading**: prints `changing `helper` affects nothing else`, exit **0** |
| `callers <name> --tree` | ⚠️ Prints just the name, exit 0 |
| `path main helper` | ❌ Confusing error: `` `main` is at ./app/main.py:4, outside any function `` — it couldn't find a function called `main`, so it searched for the *text* "main", found it outside any (Rust) function, and gave up |
| `trak` (interactive) | ⚠️ Header says `Repository: py (0 files)` (it counts `.rs` files only). Typing a name gives "`x` is at file:line, outside any function, so nothing calls it" |

**Mixed repos** (Rust + Python + JS + Go): the Rust part gets the full graph,
the rest gets text search only. `path main core` ✅ across Rust files;
`path start "render failed"` on a JS function ❌ (same confusing
"outside any function" error as above).

**Which extensions count as "source"** (searched, and shown in
`architecture`): `c cc cpp cs css cxx dart ex go h hh hpp hxx html java js
jsx json kt lua md php pl py rb rs scala scss sh sql swift toml ts tsx txt
vue yaml yml zig`. Anything else (`.svelte`, `.proto`, `.graphql`, `.mdx`,
`Makefile`, `Dockerfile`, files without an extension, ...) is invisible to
every command.

---

## 3. Where trak thinks the repository is

trak walks up from the current directory to the first folder containing
`.git` (a directory or a file, so worktrees count) and treats that as the
root.

| Situation | Behaviour |
| --- | --- |
| Run from any subfolder of a git repo | ✅ Analyses the whole repo (same result as from the root) |
| **Not in a git repo at all** | ⚠️ The root becomes the **current directory**, not the project root. Run from `myproj/src` and only `src/` is analysed. `cd` to the top-level folder first |
| git repo with **no commits yet** | ⚠️ Graph commands ✅. `history` ❌ `fatal: your current branch 'master' does not have any commits yet`. `blame` ❌ `fatal: no such ref: HEAD`. `diff` says "clean" |
| Empty repo / no source files | ✅ `0 source files · 0 symbols · 0 functions`; everything else says "nothing found" |
| Nested repo (a folder with its own `.git` inside yours) | ⚠️ From inside it, trak stops at the inner `.git`. From outside, the inner repo's files are included in the outer analysis |

---

## 4. Which files are read (and which silently aren't)

### Folders that are always skipped, at any depth

`.git` `.git-rewrite` `.idea` `.vscode` `.venv` `__pycache__` `build` `dist`
`node_modules` `target` `vendor`

⚠️ This is by **name**, so a real source folder called `build/`, `dist/` or
`vendor/` disappears completely: functions defined there don't exist, and
calls into them are dropped from the graph. (Tested: `from_build()` and
`from_vendor()` were missing from `calls main`.)

### `tests/fixtures/` is skipped

Any path containing `tests/` followed directly by `fixtures/` is excluded
from both search and the graph (so sample projects used by tests don't pollute
the real call graph). A top-level `fixtures/` or `src/fixtures/` is kept.
Display quirk: `architecture` still draws `tests/fixtures/f.rs` in the tree
but says `(0 files)`.

### Search and the call graph don't agree on what's visible

| File | Search (`search`, `relevant`, `log`/`error`/`panic`, `path` by message) | Call graph (`fn`, `calls`, `callers`, ...) |
| --- | --- | --- |
| Listed in `.gitignore` (e.g. generated code) | ❌ invisible | ✅ parsed |
| Inside a hidden folder (`.hidden/`, `.github/`, `.cargo/`) | ❌ invisible | ✅ parsed |
| Not valid UTF-8 (e.g. Latin-1 bytes) | ✅ found | ❌ skipped entirely (its functions don't exist) |
| Random binary bytes with a `.rs` name | no crash, no results | no crash, no results |

Consequence: `trak error "msg"` can say "nothing found" for a message printed
by a gitignored generated file, even though `trak fn` sees that file's
functions.

### Symlinks

⚠️ Symlinked files are followed and parsed as if they were separate files. A
`link.rs → src/main.rs` symlink produced a second `main`, and `calls main`
then picked the symlink's copy.

---

## 5. Rust: what the parser understands

trak doesn't compile your code. It reads `.rs` files line by line and matches
calls **by function name**. Tested with rustfmt-formatted code:

### ✅ Resolved correctly

- Plain calls `leaf()`, calls through paths `crate::x::f()`, `super::f()`,
  `inline_mod::f()`, `module::f()`
- `Self::f()` (fixed in 0.1.0) and `Type::f()` for types declared in the same file
- `self.method()`, and `value.method()` when the method name isn't a std name (see below)
- Trait methods, including trait default methods calling `self.other()`
- Calls into a dyn trait: `s.describe()` → the trait's `describe`
- Imports and aliases: `use net::connect as dial; dial()` → `connect`
- Imports from another crate in the same workspace (kept); imports from
  outside crates (dropped)
- Calls inside macro *arguments*: `my_macro!(hidden_call())` → `hidden_call`
- Closures: `|x| closure_target(x)` is attributed to the enclosing function
- Nested functions: `fn outer() { fn nested() {...} nested(); }`
- Direct recursion (`recursive → recursive`) and mutual recursion
  (`ping → pong → ping`); trees and impact terminate on cycles
- Multi-line signatures, `where` clauses, `async fn`, `const fn`,
  `unsafe extern "C" fn`, `pub(crate) fn`
- Strings, raw strings, char literals and comments don't produce fake calls
  or fake declarations
- CRLF (Windows) line endings
- `#[cfg(test)]` / `#[test]` functions are included (so they show up in
  `callers` and `impact`, which can be noisy)

### ❌ Not resolved

| Pattern | What happens |
| --- | --- |
| **Two functions with the same name** (`new`, `run`, `process`, `main` in `src/main.rs` and `src/bin/x.rs`, trait impls on different types) | The graph is keyed by bare name. `fn`/`calls`/`open`/`calls --tree` silently pick the **first** match. `callers`/`impact` merge **all** same-name functions: if `net::process` and `main.rs::process` both exist, `callers process` lists callers of either. Biggest accuracy limitation in practice. Interactive mode does list every definition to pick from ✅ |
| Your own method named like a std method, called on a variable: `store.get()`, `store.len()`, `x.push()`, `x.clear()`, `x.flush()`... | ❌ Dropped (it assumes the receiver is a std type). Same method via `self.get()` ✅ works. Non-std names (`store.fetch()`) ✅ work |
| Turbofish calls: `generic::<u8>(1)` | ❌ Edge missing |
| Macro bodies: `macro_rules! m { () => { leaf() } }` then `m!()` | ❌ The call inside the macro body isn't connected to the caller of `m!()` |
| Function passed as a value: `thread::spawn(worker)`, `.map(parse)` | ❌ Not a call (no parentheses) |
| Raw identifiers: `fn r#match()` | ❌ `symbol match` / `symbol r#match` → not found |
| Non-ASCII identifiers: `fn naïve()` | ❌ not found |
| **Several items on one line**: `fn a() { b(); } fn b() { a(); }`, one-line `impl X { fn f() {} fn g() {} }` | ❌ Only the first function on the line gets its calls right; later ones lose calls or get them attributed to the wrong function. rustfmt'd code is fine |
| Trait method *declaration* (`fn area(&self) -> f64;`) plus its impl | ⚠️ Can show up twice in `callers` output (`leaf` ← `area`, `area`) |
| Calls generated by derives, `?` conversions (`From`), operators (`Add`), `Drop`, `Deref` | ❌ Invisible (no type information) |
| Generic / trait-object dispatch to a *specific* impl | ⚠️ Goes to every function with that name, not the right impl |

---

## 6. Command-by-command edge cases

### `search`

- ⚠️ **Case-insensitive**: `search NEEDLE` matches `needle`
- ⚠️ **Capped at 200 hits**, silently: the header just says `200 hit(s)`, with no "truncated" note
- ⚠️ `search ""` (empty query) matches every line → 200 hits
- ✅ Regex characters are literal: `search "a.b*("` searches for exactly that text
- Searches Markdown/TOML/YAML/etc. too, so docs that quote your code show up

### `symbol`

- Shows the declaration kind (`[fn]`, `[struct]`, `[impl]`, `[trait]`, `[macro]`, ...) and *every* declaration with that name
- ❌ Raw / non-ASCII identifiers aren't found

### `fn`, `calls`

- Unknown name → `error: nothing found for function` (exit 1)
- ⚠️ Duplicate names: `fn` lists every definition, but the callee edges are from **one** of them; `calls` silently uses the first one found
- A function that calls nothing in the repo → `0 call(s)` (exit 0)

### `callers`, `impact`

- ⚠️ **Unknown name is not an error**: `callers zz_typo` → `no callers of `zz_typo`` (exit 0); `impact zz_typo` → `affects nothing else` (exit 0). A typo looks like "safe to change". Check with `trak symbol <name>` first
- ⚠️ Same-name functions are merged (see §5)
- ⚠️ No limit on output: in a large tree, `callers new` printed 86,312 callers and `impact from_str` printed 143,970 functions
- Test functions are included, which inflates `impact`

### `--tree` (on `calls` and `callers`)

- ⚠️ Each function is printed **once per tree**. If it's reachable by two routes, the second route is hidden. Example: `callers leaf --tree` showed `dyn_call` without its caller `main`, because `main` had already appeared under another branch. `callers dyn_call` on its own does show `main`
- Cycles terminate ✅

### `path`

- **Resolution order:** if the argument is a function name the repo defines, it's a function; **otherwise it's treated as a message** to search for. So a typo'd or non-Rust function name gets the message error: `` `main` is at file:line, outside any function `` or `nothing found for function or message`
- Order matters: `path main from_dir` ✅, `path from_dir main` → `no call path` (exit **0**)
- `{name}` placeholders in a message match anything: `"no callers of {name}"` matches ``"no callers of `{name}`"``
- ✅ (fixed in 0.1.0) A message that also appears in docs/comments still resolves to the function that emits it. Only when *no* function contains it do you get "outside any function"
- When several functions emit the message, every reachable route is listed, shortest first
- ⚠️ Same-name functions: the path is computed on names, so a chain can jump between two different functions called `run`

### `refs`

- Definition + every function that calls it, grouped by file
- Same name-based caveats as `callers`

### `relevant`, `log`, `error`, `panic`

- All four are the **same** command with a different heading
- Built on search, so: case-insensitive, 200-hit cap, gitignored/hidden files invisible
- No `--format` (terminal only, rejected by clap)
- ⚠️ `called from:` lists names, including duplicates (`run, run, run, ...`) when several same-name functions call it

### `architecture`

- Counts every source extension in "source files", but symbols/functions only for `.rs`
- ⚠️ Slow on big trees (it parses everything): 62 s on a 63k-file tree
- Terminal only

### `history`

- It's `git log --oneline -S <name>`: commits whose diff **added or removed the text** anywhere, case-sensitive
- ⚠️ Not tied to the function. A short name (`run`, `new`) matches commits that touched any `run`, comment or string
- Nothing found → `no git history mentions `x`` (exit 0)
- Outside git → raw git error (exit 1)

### `blame`

- `file:line` → one line. `file` alone → whole file
- ⚠️ **Paths are relative to the repo root, not your current directory.** From `src/`, `blame main.rs:2` ❌ fails; `blame src/main.rs:2` ✅. Absolute paths ✅ and `./src/main.rs:2` ✅ also work
- ⚠️ `file:0` silently blames the **whole file**
- Errors are passed straight through from git: `file Cargo.toml has only 33 lines`, `no such path 'x' in HEAD`, `not a git repository`
- ⚠️ Files that exist but were never committed → `no such path ... in HEAD`
- Paths with spaces ✅ (`blame "my dir/s.rs:1"`)

### `diff`

- ⚠️ It's plain `git diff`: **unstaged changes only**
- ⚠️ Staged-only changes print `no diff (clean working tree)`, which is wrong
- ⚠️ New untracked files aren't shown
- Outside git → git's full `--no-index` usage text as the error (exit 1)

### `open`

- Name → first definition with that name (⚠️ for a trait method, that's the trait *declaration*, not the impl)
- `path`, `path:line`, `path:line:col` ✅
- Missing file → `nothing found` (exit 1). A line past the end of the file is passed to the editor as-is (exit 0)
- Editor choice: `TRAK_EDITOR` → the editor you're running inside → `$VISUAL` → `$EDITOR` → first installed of Zed, VS Code/Cursor, Neovim, Vim, Helix
- ⚠️ A custom `TRAK_EDITOR` is called as `<editor> +LINE <file>`: the **column is dropped**, and editors that don't understand `+LINE` will misbehave
- ⚠️ **Launch failures aren't detected**: `TRAK_EDITOR=/nonexistent` still printed `opened ... in Custom` and exited **0**
- ⚠️ With no editor configured and nothing installed except Vim, trak starts Vim. **Without a terminal** (scripts, CI, an agent) that **hangs** until killed

### Interactive mode (`trak` / `trak interactive`)

- Rust only. Header counts `.rs` files only (`py (0 files)` in a Python repo)
- ✅ Duplicate names are handled properly: "20 functions for `run`", pick by number
- ✅ Regex characters → `nothing found`, no crash. Invalid UTF-8 input doesn't kill the session
- Bare `trak` without a terminal prints usage and exits **2** (won't hang a script). Use `trak interactive` to script it: `printf 'leaf\n4\nq\n' | trak interactive`
- Match list capped at 20, source view at 60 lines, traces at 200 lines
- ⚠️ Trees (`4`, `5`) have the same print-each-function-once behaviour as `--tree`

---

## 7. Exit codes

| Code | When |
| --- | --- |
| `0` | Success, including `no call path`, `0 call(s)`, `no callers of X`, `affects nothing else`, `no git history mentions X`, `no diff` — and including unknown names for `callers` / `impact` |
| `1` | `nothing found for ...`, "outside any function", any git failure, missing file for `open` |
| `2` | Bad arguments (unknown flag such as `relevant --format`, missing argument), or bare `trak` without a terminal |

If you script around trak: check `callers`/`impact` names with
`trak symbol <name>` first, because exit 0 doesn't mean the name exists.

---

## 8. Performance

Every command re-reads and re-parses the whole repository; there's no cache
or index between runs.

| Repo | `architecture` | `callers` / `impact` | `search` |
| --- | --- | --- | --- |
| trak itself (~70 files) | instant | instant | instant |
| ~/.cargo/registry/src (2.2 GB, 63k `.rs` files, 1.15M functions) | 62 s | 37–39 s | 0.45 s (stops at 200 hits) |

Interactive mode parses once per session and caches, so it's the fast option
for exploring a big repo.

---

## 9. Bugs and gaps worth fixing next

In rough order of how likely they are to mislead someone:

1. `callers` / `impact` on an unknown name should be `nothing found` (exit 1), not "no callers" / "affects nothing"
2. Same-name functions are merged or picked arbitrarily. Key the graph by `file + name` (or module path), and make `fn`/`calls`/`open` list the choices like interactive mode does
3. `diff` should include staged changes (`git diff HEAD`) or at least not say "clean working tree" when there are staged changes
4. `open` should report a failed editor launch and not fall back to a terminal editor when there's no terminal
5. Search should use the same file rules as the parser (gitignored/hidden files), or at least say so
6. Search should say when it hit the 200 cap
7. `path` with a name that isn't a function should say "no function named X" before falling back to message search
8. `blame` should resolve paths relative to the current directory
9. Non-Rust repos: print a one-line "trak only understands Rust; showing text results" note instead of empty/misleading graph answers
10. Parser gaps: turbofish calls, raw/non-ASCII identifiers, std-named methods on local types, several items on one line
11. `--tree` could mark already-shown nodes (`main (see above)`) instead of hiding them
12. A cache/index for big repos
