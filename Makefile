.PHONY: setup fmt check test run

setup:
	chmod +x .githooks/pre-commit
	git config core.hooksPath .githooks
	@echo "Git hooks installed."

fmt:
	cargo fmt --all

check:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

run:
	cargo run
