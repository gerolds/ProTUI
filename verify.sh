#!/bin/sh
# ADR: verify.cmd for macOS and Linux, in POSIX sh: fmt check, Clippy with denied warnings, tests, headless smoke, all in the repository's target/; it stops at the first failure with its status.
set -e
cd -- "$(dirname -- "$0")"
CARGO_TARGET_DIR="$PWD/target"
export CARGO_TARGET_DIR
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
exec cargo run --locked -p prototype -- --smoke
