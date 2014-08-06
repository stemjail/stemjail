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

extern crate native;

use self::fs::dup;
use self::native::io::file::FileDesc;
use std::io;

#[path = "../../ffi/fs.rs" ]
mod fs;

pub struct Stdio {
    pub stdin: FileDesc,
    pub stdout: FileDesc,
    pub stderr: FileDesc,
}

impl Stdio {
    pub fn new(fd: FileDesc) -> io::IoResult<Stdio> {
        Ok(Stdio {
            // Can't close on drop because of the io::Command FD auto-closing
            stdin: try!(dup(&fd, false)),
            stdout: try!(dup(&fd, false)),
            stderr: try!(dup(&fd, false)),
        })
    }
}
