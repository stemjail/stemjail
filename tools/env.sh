#!/bin/sh

RUSTC_MODE="${RUSTC_MODE:-release}"
DIR="$(dirname -- "$(readlink -f "$0")")/../../stemshim/target/${RUSTC_MODE}"

export DISPLAY=:1
export LD_LIBRARY_PATH="${DIR}"
export LD_PRELOAD="${DIR}/hook-open.so"

exec "$@"
