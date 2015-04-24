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

use ffi::net;
use self::iohandle::FileDesc;
use self::libc::{size_t, c_void};
use std::old_io as io;
use std::old_io::net::pipe::UnixStream;
use std::os::unix::{AsRawFd, Fd};

#[repr(C)]
struct FdPadding {
    pub fd: Fd,
    _padding: [u8; 2],
}

impl FdPadding {
    pub fn new(fd: Fd) -> FdPadding {
        FdPadding {
            fd: fd,
            _padding: [0, 0],
        }
    }
}

pub fn recv_fd(stream: &mut UnixStream, iov_expect: Vec<u8>) -> io::IoResult<FileDesc> {
    let fd = FdPadding::new(-1 as Fd);
    match net::recvmsg(stream, iov_expect.len(), fd) {
        // TODO: Check size?
        Ok((_, iov_recv, data)) => {
            if iov_recv != iov_expect {
                return Err(io::standard_error(io::OtherIoError));
            }
            Ok(FileDesc::new(data.fd, true))
        }
        Err(e) => Err(e),
    }
}

pub fn send_fd(stream: &mut UnixStream, id: &[u8], fd: &AsRawFd) -> io::IoResult<()> {
    let iov = net::Iovec {
        iov_base: id.as_ptr() as *const c_void,
        iov_len: id.len() as size_t,
    };
    let fda = FdPadding::new(fd.as_raw_fd());
    let ctrl = net::Cmsghdr::new(net::SOL_SOCKET, net::Scm::Rights, fda);
    let msg = net::Msghdr::new(None, vec!(iov), &ctrl, None);
    match net::sendmsg(stream, msg) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
