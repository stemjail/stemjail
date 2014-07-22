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

extern crate libc;

use std::{io, os};

mod raw {
    extern crate libc;

    use self::libc::{c_char, c_int};

    extern {
        pub fn chroot(path: *const c_char) -> c_int;
    }
}

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

fn chdir(dir: &Path) -> io::IoResult<()> {
    match os::change_dir(dir) {
        true => Ok(()),
        false => Err(io::standard_error(io::OtherIoError)),
    }
}

#[allow(dead_code)]
pub fn chroot(path: &Path) -> io::IoResult<()> {
    try!(chdir(path));
    let p = path2str!(path);
    p.with_c_str(|s| {
        match unsafe { raw::chroot(s) } {
            0 => Ok(()),
            _ => Err(io::IoError::last_error()),
        }
    })
}
