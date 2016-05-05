#!/bin/sh

RUSTC_MODE="${RUSTC_MODE:-release}"
DIR_BASE="$(dirname -- "$(readlink -f -- "$0")")/.."
cd "${DIR_BASE}"

STEMJAIL_LIB_SHIM_PATH="${DIR_BASE}/../stemshim/target/${RUSTC_MODE}/hook-open.so" \
	LD_LIBRARY_PATH="${DIR_BASE}/target/${RUSTC_MODE}/deps" \
	RUST_LOG=stemjail=debug,portal=debug \
	"${DIR_BASE}/target/${RUSTC_MODE}/portal" "$@"
