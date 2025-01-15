#!/usr/bin/env bash
set -euo pipefail;

MYSELF=$(realpath "$0");
MYDIR=$(dirname "$MYSELF");

FLAGS=${FLAGS:-"-Dclippy::pedantic"}

set -x;

(
    cd "$MYDIR"

    # shellcheck disable=2086
    cargo clippy -- $FLAGS
    cargo test

    # shellcheck disable=2086
    cargo clippy --no-default-features -- $FLAGS
    # tests without std dont pass for now bcs MemoryStore is used in generative
    # cargo test --no-default-features

    # shellcheck disable=2086
    cargo clippy --all-features -- $FLAGS
    cargo test --all-features

    # I'd run the http example if networking wasn't so unreliable...
    # shellcheck disable=2086
    cargo clippy --example http -- $FLAGS
    # shellcheck disable=2086
    cargo clippy --example http-multithread -- $FLAGS
)
