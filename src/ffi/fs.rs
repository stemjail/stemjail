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

extern crate iohandle;
extern crate libc;

use self::iohandle::FileDesc;
use self::libc::c_uint;
use self::libc::types::os::arch::posix88::{dev_t, mode_t};
use std::ffi::CString;
use std::io;
use std::os::unix::{AsRawFd, Fd};

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
pub enum Node {
    Block(Dev),
    Character(Dev),
    Fifo,
    Regular,
    Socket,
}

impl Node {
    fn get_stat(&self) -> mode_t {
        match *self {
            Node::Block(..) => self::libc::S_IFBLK,
            Node::Character(..) => self::libc::S_IFCHR,
            Node::Fifo => self::libc::S_IFIFO,
            Node::Regular => self::libc::S_IFREG,
            // FIXME: Missing libc::S_ISOCK
            // From Linux v3.14 include/uapi/linux/stat.h
            #[cfg(target_arch = "x86_64")]
            Node::Socket => 0o140000,
        }
    }

    fn get_dev(&self) -> dev_t {
        match *self {
            Node::Block(d) => d.makedev(),
            Node::Character(d) => d.makedev(),
            _ => 0,
        }
    }
}

#[allow(dead_code)]
pub fn dup(fd: &AsRawFd, close_on_drop: bool) -> io::IoResult<FileDesc> {
    match unsafe { self::libc::funcs::posix88::unistd::dup(fd.as_raw_fd()) } {
        -1 => Err(io::IoError::last_error()),
        n => Ok(FileDesc::new(n as Fd, close_on_drop)),
    }
}

// TODO: Set and restore umask, or return an error if permissions are masked
#[allow(dead_code)]
pub fn mknod(path: &Path, nodetype: &Node, permission: &io::FilePermission) -> io::IoResult<()> {
    let path = CString::from_slice(path.as_vec());
    let mode = nodetype.get_stat() | permission.bits();
    match unsafe { raw::mknod(path.as_ptr(), mode, nodetype.get_dev()) } {
        0 => Ok(()),
        _ => return Err(io::IoError::last_error()),
    }
}
