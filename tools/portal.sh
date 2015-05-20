#!/bin/sh

DIR_BASE="$(readlink -f -- "$(dirname -- "$0")/..")"

LD_LIBRARY_PATH="${DIR_BASE}/target/debug/deps" RUST_LOG=stemjail=debug,portal=debug "${DIR_BASE}/target/debug/portal" "$@"
