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
extern crate native;

use self::libc::c_uint;
use self::libc::types::os::arch::posix88::{dev_t, mode_t};
use self::native::io::file::{fd_t, FileDesc};
use std::io;

mod raw {
    extern crate libc;

    use self::libc::{c_char, c_int, c_uint, c_ulonglong};
    use self::libc::types::os::arch::posix88::{dev_t, mode_t};

    extern {
        pub fn gnu_dev_makedev(major: c_uint, minor: c_uint) -> c_ulonglong;
        pub fn mknod(pathname: *const c_char, mode: mode_t, dev: dev_t) -> c_int;
    }
}

pub struct Dev {
    pub major: c_uint,
    pub minor: c_uint,
}

impl Dev {
    pub fn makedev(&self) -> dev_t {
        unsafe { raw::gnu_dev_makedev(self.major, self.minor) as dev_t }
    }
}

#[allow(dead_code)]
pub enum NodeType {
    Block(Dev),
    Character(Dev),
    Fifo,
    Regular,
    Socket,
}

impl NodeType {
    fn get_stat(&self) -> mode_t {
        match *self {
            Block(..) => self::libc::S_IFBLK,
            Character(..) => self::libc::S_IFCHR,
            Fifo => self::libc::S_IFIFO,
            Regular => self::libc::S_IFREG,
            // FIXME: Missing libc::S_ISOCK
            // From Linux v3.14 include/uapi/linux/stat.h
            #[cfg(target_arch = "x86_64")]
            Socket => 0o140000,
        }
    }

    fn get_dev(&self) -> dev_t {
        match *self {
            Block(d) => d.makedev(),
            Character(d) => d.makedev(),
            _ => 0,
        }
    }
}

#[allow(dead_code)]
pub fn dup(fd: &FileDesc, close_on_drop: bool) -> io::IoResult<FileDesc> {
    match unsafe { self::libc::funcs::posix88::unistd::dup(fd.fd()) } {
        -1 => Err(io::IoError::last_error()),
        n => Ok(FileDesc::new(n as fd_t, close_on_drop)),
    }
}

// TODO: Set and restore umask, or return an error if permissions are masked
#[allow(dead_code)]
pub fn mknod(path: &Path, nodetype: &NodeType, permission: &io::FilePermission) -> io::IoResult<()> {
    let path = path2str!(path);
    let mode = nodetype.get_stat() | permission.bits();
    path.with_c_str(|p| {
        match unsafe { raw::mknod(p, mode, nodetype.get_dev()) } {
            0 => Ok(()),
            _ => return Err(io::IoError::last_error()),
        }
    })
}
