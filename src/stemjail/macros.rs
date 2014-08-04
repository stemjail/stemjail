// Copyright (C) 2014 Mickaël Salaün
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, version 3 of the License.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

#![macro_escape]

macro_rules! path2str(
    ($path: expr) => (
        match $path.as_str() {
            Some(p) => p,
            None => return Err(io::IoError {
                kind: io::PathDoesntExist,
                desc: "path conversion fail",
                detail: None,
            }),
        }
    );
)
