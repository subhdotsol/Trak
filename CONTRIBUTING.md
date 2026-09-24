# Contributing to Trak

Thank you for contributing. This guide covers the development workflow, the quality gates that run in CI, and the issue and pull request templates.

## Repository layout

Trak is a Cargo workspace. The CLI lives in `crates/trak` and delegates all logic to the backend crates: `trak-core`, `trak-rust`, `trak-analysis`, `trak-search`, `trak-git`, `trak-output`, and `trak-editor`.

A design rule applies to every change: analysis logic belongs in the backend crates. Clap definitions and command dispatch belong only at the CLI boundary in `crates/trak`.

## Setting up

```sh
git clone git@github.com:subhdotsol/trak.git
cd trak
cargo build --workspace
```

You need a stable Rust toolchain installed via [rustup](https://rustup.rs/).

## Running the checks

CI runs the exact commands below on every push and pull request. Run them locally before pushing to avoid a failed CI run.

### Format

```sh
cargo fmt --all -- --check
```

To apply formatting instead of checking:

```sh
cargo fmt --all
```

### Lint

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Treat every clippy warning as an error. The `-D warnings` flag mirrors CI.

### Tests

```sh
cargo test --workspace --all-features
```

The full suite includes:

- Unit tests in `crates/trak-rust`, covering the dependency-free parser.
- Integration tests in `crates/trak/tests`, which run the compiled binary against fixture repositories.

### Full pre-push checklist

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Run all three. CI performs the same three steps plus `cargo check --workspace --all-targets`, so a green local run is a green CI run.

## Testing conventions

- New fixtures live in `crates/trak/tests/fixtures/rust/<name>/`.
- Integration tests use the shared harness in `crates/trak/tests/common/mod.rs`, which copies the fixture into a fresh temp directory before running the binary. Keep using the harness so tests stay hermetic and never escape into the workspace git root.
- Parser behavior changes in `crates/trak-rust` should ship with unit tests next to the code.

## Style

- Run `cargo fmt --all` before submitting. The repository owns no custom rustfmt configuration.
- Keep code free of needless comments. Prefer expressive names over explanatory comments.
- Keep the parser in `trak-rust` free of external dependencies.
- Wire new subcommands through `cli.rs` and `commands/` in `crates/trak`; never put clap types in backend crates.

## Commits

Use Conventional Commits:

```text
feat: add --json flag to callers
fix: drop self-calls from call graphs
refactor: pull blame into trak-git
docs: document output formats
test: cover impact analysis on fixtures
ci: run clippy with -D warnings
```

Keep each commit focused on one change and update tests in the same commit.

## Branches

- Feature branches from `main`: `feat/`, `fix/`, `docs/`, `chore/`.
- Pull requests target `main`.
- Keep pull requests small and reviewable, ideally one logical change.

## Issue template

Open a bug report or a feature request using the templates below. The same templates are also enabled on GitHub under `.github/ISSUE_TEMPLATE/`.

### Bug report

```markdown
**Describe the bug**
A clear and concise description of what is wrong.

**Environment**
- Trak version: `trak --version`
- OS: (Linux, macOS, Windows)
- Rust toolchain: `rustc --version`

**To reproduce**
Steps to reproduce:
1. Clone or create a minimal Rust repository, or point at a specific fixture.
2. Run ``trak <command> <args>``.
3. Observe the result.

**Expected behavior**
What you expected to happen.

**Actual behavior**
What actually happened, including the full terminal output.

**Additional context**
Anything else relevant: sample code, repository structure, screenshots of error output.
```

### Feature request

```markdown
**Problem**
What problem are you trying to solve? Describe the workflow that is hard or missing today.

**Proposed change**
What should Trak do? Include the command shape if you have one, for example ``trak <new-command> <name>``.

**Alternatives considered**
What workarounds you tried, and why they fall short.

**Additional context**
Anything else relevant to the design of the feature.
```

## Pull request template

```markdown
**Summary**
A short description of what this PR does and why.

**Changes**
- Bullet list of the notable changes.

**Related issue**
Fixes #<issue-number> or N/A.

**Testing**
List the commands you ran and their results, for example:

- [ ] ``cargo fmt --all -- --check``
- [ ] ``cargo clippy --workspace --all-targets --all-features -- -D warnings``
- [ ] ``cargo test --workspace --all-features``

**Checklist**
- [ ] I ran the full pre-push checklist above.
- [ ] I added or updated tests for this change.
- [ ] I updated the documentation if the CLI surface changed.
```

The same template is enabled on GitHub under `.github/PULL_REQUEST_TEMPLATE.md`.