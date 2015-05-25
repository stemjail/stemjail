#!/bin/sh

DIR_BASE="$(readlink -f -- "$(dirname -- "$0")/..")"

ARGS="run -t pro-bank"
if [ $# -ne 0 ]; then
	ARGS="$*"
fi

LD_LIBRARY_PATH="${DIR_BASE}/target/debug/deps" RUST_LOG=stemjail=debug,kage=debug "${DIR_BASE}/target/debug/kage" ${ARGS}
