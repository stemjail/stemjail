#!/bin/sh

# Copyright (C) 2014 Mickaël Salaün
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

set -e

DIR="$1"

usage() {
	echo "usage: $0 <chroot-dir>" >&2
	exit 1
}

if [ -z "${DIR}" ]; then
	usage
fi
if [ -e "${DIR}" ]; then
	if [ ! -d "${DIR}" ]; then
		usage
	fi
else
	mkdir "${DIR}"
fi

DEB="$(ls -1 busybox-static_*.deb 2>/dev/null || true)"
if [ -z "${DEB}" ]; then
	apt-get download busybox-static
elif [ -f "${DEB}" ]; then
	echo "Using ${DEB}"
else
	echo "Error: Multiple Busybox packages in the current directory." >&2
	exit 1
fi
dpkg -x busybox-static_*.deb "${DIR}"

cd "${DIR}"
./bin/busybox --install ./bin
