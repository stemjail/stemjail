#!/bin/sh

RUSTC_MODE="${RUSTC_MODE:-release}"
DIR_BASE="$(dirname -- "$(readlink -f -- "$0")")/.."

ARGS="run -t -- ${DIR_BASE}/tools/env.sh /usr/bin/setsid -c /bin/bash"
if [ $# -ne 0 ]; then
	ARGS="$*"
fi

LIB="${DIR_BASE}/target/${RUSTC_MODE}/deps"
if [ -n "${LD_LIBRARY_PATH}" ]; then
	LIB="${LD_LIBRARY_PATH}:${LIB}"
fi

LD_LIBRARY_PATH="${LIB}" RUST_LOG=stemjail=debug,kage=debug "${DIR_BASE}/target/${RUSTC_MODE}/kage" ${ARGS}
