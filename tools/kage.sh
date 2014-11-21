#!/bin/sh

DIR_BASE="$(readlink -f -- "$(dirname -- "$0")/..")"

ARGS="run foo sh"
if [ $# -ne 0 ]; then
	ARGS="$*"
fi

LD_LIBRARY_PATH="${DIR_BASE}/target/deps" RUST_LOG=stemjail=debug "${DIR_BASE}/target/kage" ${ARGS}
