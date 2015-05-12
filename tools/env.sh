#!/bin/sh

DIR="$(dirname -- "$(readlink -f "$0")")/../../stemshim/target/debug"

export DISPLAY=:1
export LD_LIBRARY_PATH="${DIR}"
export LD_PRELOAD="${DIR}/hook-open.so"

exec "$@"
