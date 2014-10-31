#!/bin/sh

DIR_BASE="$(readlink -f -- "$(dirname -- "$0")/..")"

LD_LIBRARY_PATH="${DIR_BASE}/target/deps" RUST_LOG=stemjail=debug "${DIR_BASE}/target/portal"