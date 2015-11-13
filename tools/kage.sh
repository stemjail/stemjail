#!/bin/sh

RUSTC_MODE="${RUSTC_MODE:-release}"
DIR_BASE="$(dirname -- "$(readlink -f -- "$0")")/.."

ARGS="run -t -- /usr/bin/setsid -c /bin/bash"
if [ $# -ne 0 ]; then
	ARGS="$*"
fi

LD_LIBRARY_PATH="${DIR_BASE}/target/${RUSTC_MODE}/deps" RUST_LOG=stemjail=debug,kage=debug "${DIR_BASE}/target/${RUSTC_MODE}/kage" ${ARGS}
