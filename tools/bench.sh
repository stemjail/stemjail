#!/bin/bash

# Copyright (C) 2015 Mickaël Salaün
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Lesser General Public License as published by
# the Free Software Foundation, version 3 of the License.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Lesser General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.

set -eu

TIMES="10"

TESTS="bench_gunzip bench_untar bench_zip bench_build1 bench_build4"

# Must be a tar.gz file format
TARBALL="linux-4.4.tar.gz"

if [ ! -f "${TARBALL}" ]; then
	echo "ERROR: The tarball ${TARBALL} is missing." >&2
	exit 1
fi

check_time() {
	sync
	# Format: kernelland time, userland time
	exec 9>&1
	(
		echo "$@ ; sync" | /usr/bin/time --format '%S %U %E' --output "/proc/self/fd/9" --  bash --noprofile --norc > /dev/null 2>&1
	)
	exec 9>&-
}

bench_gunzip() {
	rm -f -- "${TARBALL%%.gz}" 2> /dev/null || true
	check_time gunzip -k -- "${TARBALL}"
}

bench_untar() {
	rm -rf -- "${TARBALL%%.tar.gz}" 2> /dev/null || true
	check_time tar -xf "${TARBALL%%.gz}"
}

bench_zip() {
	local dir="${TARBALL%%.tar.gz}"
	local file="${dir}.zip"
	rm -f -- "${file}" 2> /dev/null || true
	check_time zip -qr "${file}" "${dir}"
}

bench_build() {
	local jobs="$1"
	local data="${TARBALL%%.tar.gz}"
	if [ ! -d "${data}" ]; then
		tar -xf "${TARBALL}"
	fi
	pushd "${data}" > /dev/null
	make mrproper &> /dev/null
	make defconfig ARCH=x86_64 &> /dev/null

	check_time make -j "${jobs}" ARCH=x86_64
	popd > /dev/null
}

bench_build1() {
	bench_build 1
}

bench_build4() {
	bench_build 4
}

do_bench() {
	local func="$1"

	# Kernelland time
	local kt=0
	# Userland time
	local ut=0
	# Elapsed real time
	local et=0
	local all=

	for i in $(seq 1 "${TIMES}"); do
		all="$(${func})"
		echo "${func}/time: ${all}" >&2
		kt="$(echo "${all}" | awk -v c="${kt}" '{print $1 " + " c}' | bc -l)"
		ut="$(echo "${all}" | awk -v c="${ut}" '{print $2 " + " c}' | bc -l)"
		et="$(echo "${all}" | sed -r 's/ ([0-9]+):/ (60*\1)+/g' | awk -v c="${et}" '{print $3 " + " c}' | bc -l)"
	done

	# Average
	local kta=0
	local uta=0
	local eta=0
	kta="$(echo "scale=2; ${kt} / ${TIMES}" | bc -l)"
	uta="$(echo "scale=2; ${ut} / ${TIMES}" | bc -l)"
	eta="$(echo "scale=2; ${et} / ${TIMES}" | bc -l)"

	echo "${func} ${kta} ${uta} ${eta}"
}

echo "Starting $(basename -- "$0") in $(pwd)"
for func in ${TESTS}; do
	do_bench "${func}"
done
