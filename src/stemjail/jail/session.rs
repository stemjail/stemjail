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

extern crate pty;

use self::pty::TtyProxy;
use std::io;
use std::io::fs::{FileDesc, fd_t};

#[path = "../../ffi/fs.rs" ]
mod fs;

pub struct Stdio {
    tty: TtyProxy,
}

impl Stdio {
    pub fn new(fd: FileDesc) -> io::IoResult<Stdio> {
        let tty = try!(TtyProxy::new(fd));
        Ok(Stdio {
            tty: tty,
        })
    }

    // Take care of the return FD lifetime
    pub unsafe fn stdin(&self) -> fd_t {
        self.tty.pty.slave.fd()
    }

    pub unsafe fn stdout(&self) -> fd_t {
        self.tty.pty.slave.fd()
    }

    pub unsafe fn stderr(&self) -> fd_t {
        self.tty.pty.slave.fd()
    }
}
