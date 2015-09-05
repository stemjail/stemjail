// Copyright (C) 2014-2015 Mickaël Salaün
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

use std::fs::File;
use std::io;
use std::path::Path;
use tty::{FileDesc, TtyServer};

pub struct SessionIo {
    tty: TtyServer,
}

impl SessionIo {
    pub fn new(fd: &FileDesc) -> io::Result<SessionIo> {
        let tty = try!(TtyServer::new(Some(fd)));
        Ok(SessionIo {
            tty: tty,
        })
    }

    // Take care of the return FD lifetime
    pub fn take_slave_fd(&mut self) -> Option<File> {
        self.tty.take_slave()
    }

    pub fn get_master(&self) -> &File {
        self.tty.get_master()
    }
}

impl AsRef<Path> for SessionIo {
    fn as_ref(&self) -> &Path {
        self.tty.as_ref()
    }
}
