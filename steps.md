# Publishing trak v0.1.0 to crates.io

trak is a workspace of 8 crates. The binary crate `trak` depends on the 7
library crates, so **all 8 must be published**. crates.io does not accept
`path`-only dependencies. Publish order (dependencies first):

```
trak-core, trak-output          (no internal deps)
trak-rust, trak-analysis, trak-search, trak-git, trak-editor   (need trak-core)
trak                            (needs all of the above)
```

`cargo publish --workspace` (stable since cargo 1.90; this repo pins 1.99.1)
works out this order for you and waits for each crate to show up in the index
before publishing the next one.

> A publish is **permanent**. A version can never be overwritten or deleted,
> only yanked. Do the dry run (step 5) first.

---

## Already done (in the working tree, not committed yet)

These were required to publish and are already in place:

- [x] Every internal dependency now has a version: the root `Cargo.toml` has a
      `[workspace.dependencies]` table (`path` + `version = "0.1.0"`), and
      each crate uses `trak-core.workspace = true` etc.
- [x] crates.io metadata on every crate: `description` (required), `license`,
      `repository`, `homepage`, `readme`, `keywords` (max 5), `categories`.
- [x] `rust-version = "1.88"` (the MSRV). The code uses let-chains, which need
      1.88. Checked: builds on 1.88, fails on 1.85.
- [x] All 8 names (`trak`, `trak-core`, `trak-rust`, `trak-analysis`,
      `trak-search`, `trak-git`, `trak-output`, `trak-editor`) were free on
      crates.io at the time of writing (API returned 404).
- [x] `cargo publish --workspace --dry-run --allow-dirty` packages, compiles,
      and reaches the upload step for all 8 crates with no warnings.
- [x] Bug fixes found while checking `command.md` (see the end of this file).
- [x] `LICENSE` (MIT, subhdotsol) in the repo root and copied into every crate
      (step 2), and README install instructions + badges (step 3).

`rust-toolchain.toml` (1.99.1) only affects building from this repo. It is
not packaged into the crates, so people running `cargo install trak` use their
own toolchain, as long as it is at least 1.88.

---

## Step 1 — crates.io account and token (one time)

1. Go to <https://crates.io> and **Log in with GitHub**.
2. Open <https://crates.io/settings/profile>, add your email, and **verify**
   it from the link in the email. Publishing fails without a verified email.
3. Open <https://crates.io/settings/tokens> → **New Token**.
   - Name: e.g. `trak-publish`
   - Expiration: whatever you like (e.g. 30 days)
   - Scopes: `publish-new` **and** `publish-update`
   - Crates: leave empty (or `trak*`) — the crates don't exist yet.
   - Copy the token; it is shown only once.
4. Log in locally:

   ```sh
   cargo login
   # paste the token when prompted
   ```

   The token is stored in `~/.cargo/credentials.toml`. That file already
   exists on this machine. If you're unsure whether it holds a valid token,
   just run `cargo login` again to overwrite it. Never commit or share it.

## Step 2 — add a LICENSE file (recommended)

`Cargo.toml` says `license = "MIT"`, which is all crates.io needs. But the MIT
license requires the license text to ship with the code, and the repo has no
`LICENSE` file. Create `LICENSE` in the repo root:

```text
MIT License

Copyright (c) 2026 <your name>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

The root `LICENSE` is outside every crate directory, so it isn't packaged on
its own. To ship it with each crate, keep `license = "MIT"` as it is and copy
the file into each crate (cargo packages every file in a crate's directory):

```sh
for c in crates/*/; do cp LICENSE "$c"; done
```

## Step 3 — update the README install instructions

Once published, people install with one command. In `README.md`, under
**Setup**, add this before the "clone the repository" steps:

```md
Install from crates.io (requires Rust 1.88+):

    cargo install trak

Or build from source:
```

Also worth adding a crates.io badge at the top:

```md
[![crates.io](https://img.shields.io/crates/v/trak.svg)](https://crates.io/crates/trak)
```

The README is the page shown on crates.io for every crate (`readme.workspace = true`).

## Step 4 — run the full CI locally

Same commands as `.github/workflows/ci.yml`:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --release
```

All of these passed at the time of writing (126 tests, 0 failed).

Optional MSRV check (only needed if you touched the code since):

```sh
rustup toolchain install 1.88 --profile minimal
cargo +1.88 check --workspace --all-targets
```

## Step 5 — commit everything, then dry run

`cargo publish` refuses to publish uncommitted changes, so commit first:

```sh
git checkout -b release/v0.1.0        # or stay on main if you push directly
git add -A
git status                            # double-check what you're committing
git commit -m "chore: prepare v0.1.0 for crates.io"
```

Then the dry run (packages + compiles each crate exactly as crates.io will,
but uploads nothing):

```sh
cargo publish --workspace --dry-run
```

You should see `Packaging` / `Verifying` / `Uploading` for all 8 crates and
`warning: aborting upload due to dry run` 8 times — that's the success case.

Optional — inspect what goes into a crate:

```sh
cargo package --list -p trak
cargo package --list -p trak-core
```

## Step 6 — push and let CI go green

```sh
git push -u origin release/v0.1.0     # then open + merge the PR into main
# or: git push origin main
```

Wait for the GitHub Actions **CI** run to pass on `main`. Publish from the
exact commit that's on `main`.

```sh
git checkout main
git pull
```

## Step 7 — publish

```sh
cargo publish --workspace
```

That uploads all 8 crates in dependency order. It takes a few minutes because
it waits for each crate to appear in the index.

**If it stops partway** (network error, or crates.io rate-limits you with
`429 Too Many Requests` / "published too many new crates" — possible because
these are 8 brand-new crates):

1. Wait the time given in the error message (usually up to ~10 minutes).
2. Check which crates made it: `https://crates.io/crates/<name>` or
   `cargo search trak --limit 20`.
3. Publish the rest **one at a time, in order**, skipping the ones already up:

   ```sh
   cargo publish -p trak-core
   cargo publish -p trak-output
   cargo publish -p trak-rust
   cargo publish -p trak-analysis
   cargo publish -p trak-search
   cargo publish -p trak-git
   cargo publish -p trak-editor
   cargo publish -p trak            # always last
   ```

   Re-running an already-published crate fails with "crate version 0.1.0 is
   already uploaded". That's harmless; move on to the next one.

## Step 8 — tag the release

```sh
git tag -a v0.1.0 -m "trak v0.1.0"
git push origin v0.1.0
```

Optional: create a GitHub release from the tag
(<https://github.com/subhdotsol/trak/releases/new>) with short release notes.

## Step 9 — verify as a user would

```sh
cargo install trak            # in any directory, NOT inside the repo
trak --version                # → trak 0.1.0
cd /some/rust/project
trak architecture
trak fn main
```

Also check:

- <https://crates.io/crates/trak> shows the README, license, and repository link.
- <https://docs.rs/trak-core> etc. build automatically within ~30 minutes
  (the `trak` binary crate has no library docs, which is expected).

## Step 10 — after the release

- Revoke or let the publish token expire if you created it just for this.
- Optional: invite a co-maintainer on every crate so you're not the only owner:
  `cargo owner --add <github-user> -p trak` (repeat per crate).
- For the next version: bump `version` in the root `[workspace.package]`
  **and** the 7 `version = "0.1.0"` entries in `[workspace.dependencies]`,
  then repeat steps 4 → 9.
- A broken release can be hidden from new installs (not deleted) with
  `cargo yank --version 0.1.0 -p trak`; undo with `--undo`.

---

## What was wrong in `command.md`, and what was fixed

Every command in `command.md` was run against this repo.

**Bugs in trak itself (fixed in code, with regression tests):**

1. **`Self::method(..)` calls were dropped from the call graph.**
   `discover` calls `Self::from_dir(..)`, but `trak calls discover` showed
   `0 call(s)`, `callers from_dir` didn't list `discover`, and
   `trak path main from_dir` found no path. Fixed in
   `crates/trak-rust/src/parse.rs` (`Self` now counts as a repository path).
2. **`trak path main "<message>"` failed whenever the message also appeared in
   docs or a comment.** It gave up with "outside any function" even though
   code emitted the message. Fixed in `crates/trak/src/commands/path.rs`.
3. **A message quoted word-for-word in docs hid the real call site.** The
   placeholder-tolerant search (`{name}` → anything) only ran when the
   literal search found nothing at all. Fixed in
   `crates/trak/src/commands/relevant.rs` (also affects `relevant`, `log`,
   `error`, `panic`). Regression test: `a_message_quoted_in_docs_still_resolves_to_its_emitter`
   with the new fixture `crates/trak/tests/fixtures/rust/quoted/`.

**Errors in the doc itself (rewritten):**

- `relevant --format ...` was documented but doesn't exist (clap rejects it).
- `log`, `error`, and `panic` existed but weren't documented.
- Examples referred to code from another project: `trak calls parse_raw`,
  `trak refs format_number`, and `trak path main "failed to read input: {error}"`
  all fail on this repo. They were replaced with examples that work here.
- The sample output didn't match what trak prints (e.g. `callers from_dir — 1 function`
  vs the real `callers of `from_dir`` / `3 caller(s)`; `symbol` shows
  `fn discover  [fn]`, not the source line; `history` has no header).
- Exit codes are now documented (`0` ok, `1` nothing found, `2` bad args).
