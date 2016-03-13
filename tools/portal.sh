#!/bin/sh

RUSTC_MODE="${RUSTC_MODE:-release}"
DIR_BASE="$(dirname -- "$(readlink -f -- "$0")")/.."
cd "${DIR_BASE}"

LD_LIBRARY_PATH="${DIR_BASE}/target/${RUSTC_MODE}/deps" RUST_LOG=stemjail=debug,portal=debug "${DIR_BASE}/target/${RUSTC_MODE}/portal" "$@"
