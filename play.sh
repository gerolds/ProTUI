#!/bin/sh
# ADR: play.cmd for macOS and Linux, in POSIX sh so sh, bash and zsh all run it; builds release into the repository's target/, since only Windows locks a running executable.
# ADR: Passes --assets and every other argument on to the host, and exits with the build's or the host's status.
set -e
root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
cargo build --release --locked -p prototype --manifest-path "$root/Cargo.toml" --target-dir "$root/target"
exec "$root/target/release/prototype" --assets "$root/assets" "$@"
