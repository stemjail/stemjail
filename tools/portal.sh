#!/bin/sh

RUSTC_MODE="${RUSTC_MODE:-release}"
DIR_BASE="$(readlink -f -- "$(dirname -- "$0")/..")"

LD_LIBRARY_PATH="${DIR_BASE}/target/${RUSTC_MODE}/deps" RUST_LOG=stemjail=debug,portal=debug "${DIR_BASE}/target/${RUSTC_MODE}/portal" "$@"
