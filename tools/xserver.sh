#!/bin/sh -e

# This is a test-only configuration without authorization handling.

: ${NAME:=jail0}

NEST=":1"

command -v Xephyr >/dev/null
command -v openbox >/dev/null

Xephyr "${NEST}" -nolisten tcp -resizeable -title "${NAME}" &
DISPLAY="${NEST}" openbox &

wait %1
