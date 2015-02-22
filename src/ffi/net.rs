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

use self::libc::{c_int, size_t, ssize_t, c_uint, c_void};
use std::marker::PhantomData;
use std::mem::{size_of, size_of_val};
use std::mem::transmute;
use std::old_io as io;
use std::os::unix::AsRawFd;
use std::ptr;

pub mod raw {
    extern crate libc;

    use self::libc::{c_int, ssize_t, c_void};

    extern {
        pub fn recvmsg(sockfd: c_int, msg: *mut c_void, flags: c_int) -> ssize_t;
        pub fn sendmsg(sockfd: c_int, msg: *const c_void, flags: c_int) -> ssize_t;
    }
}

/* Got from Linux v3.14: include/x86_64-linux-gnu/bits/uio.h */
/** Structure for scatter/gather I/O. */
#[cfg(target_arch="x86_64")]
pub struct Iovec {
    /** Pointer to data. */
    pub iov_base: *const c_void,

    /** Length of data. */
    pub iov_len: size_t,
}

#[cfg(target_arch="x86_64")]
pub type Socklen = c_uint;

/* Got from Linux v3.14: include/x86_64-linux-gnu/bits/socket.h */
/** Structure describing messages sent by `sendmsg' and received by `recvmsg'. */
#[cfg(target_arch="x86_64")]
#[repr(C)]
pub struct Msghdr<T> {
    /** Address to send to/receive from. */
    msg_name: *const c_void,

    /** Length of address data. */
    msg_namelen: Socklen,

    /** Vector of data to send/receive into. */
    msg_iov: *const Iovec,

    /** Number of elements in the vector. */
    msg_iovlen: size_t,

    /** Ancillary data (eg BSD filedesc passing). */
    msg_control: *const c_void,
    _msg_control_type: PhantomData<T>,

    /** Ancillary data buffer length.
     *
     * !! The type should be Socklen but the definition of the kernel is
     * incompatible with this. */
    msg_controllen: size_t,

    /** Flags on received message. */
    // TODO: Create a dedicated flag struct
    msg_flags: c_int,
}

// Hack for safe lifetime
struct MsghdrMeta<'a, T> {
    msghdr: Msghdr<T>,
    #[allow(dead_code)]
    addr: Option<&'a [u8]>,
    #[allow(dead_code)]
    iov: Vec<Iovec>,
}

impl<T> Msghdr<T> {
    pub fn new(addr: Option<&[u8]>, iov: Vec<Iovec>, ctrl: &Cmsghdr<T>, flags: Option<c_int>) -> Msghdr<T> {
        let (msg_name, msg_namelen) = match addr {
            Some(a) => (a.as_ptr() as *const c_void, a.len() as Socklen),
            None => (ptr::null(), 0),
        };
        let msg = MsghdrMeta {
            msghdr: Msghdr {
                msg_name: msg_name,
                msg_namelen: msg_namelen,
                msg_iov: iov.as_ptr(),
                msg_iovlen: iov.len() as size_t,
                msg_control: unsafe { transmute(ctrl) },
                _msg_control_type: PhantomData,
                // The msg_controllen represent the whole space (with padding) of Cmsghdr<T>
                msg_controllen: size_of_val(ctrl) as size_t,
                msg_flags: match flags {
                    Some(f) => f,
                    None => 0,
                },
            },
            iov: iov,
            addr: addr,
        };
        msg.msghdr
    }
}

#[allow(dead_code)]
#[repr(C)]
pub enum Scm {
    /** rw: access rights (array of int) */
    Rights = 0x01,

    /** rw: struct ucred */
    Credentials = 0x02,

    /** rw: security label */
    Security = 0x03
}

/** Structure used for storage of ancillary data object information. */
/* Cmsghdr must be align with size_t */
#[cfg(target_arch="x86_64")]
#[repr(C)]
pub struct Cmsghdr<T> {
    /** Length of data in cmsg_data plus length of cmsghdr structure.
     *
     * !! The type should be Socklen but the definition of the kernel is
     * incompatible with this. */
    cmsg_len: size_t,

    /** Originating protocol. */
    cmsg_level: c_int,

    /** Protocol specific type. */
    cmsg_type: Scm,

    /** Ancillary data. */
    /* __cmsg_data must be align with size_t */
    __cmsg_data: T,
}

/* From Linux v3.14 include/uapi/asm-generic/socket.h */
pub static SOL_SOCKET: c_int = 1;

impl<T> Cmsghdr<T> {
    pub fn new(level: c_int, scm: Scm, data: T) -> Cmsghdr<T> {
        // Check alignement
        assert_eq!(size_of::<T>() % size_of::<size_t>(), 0);
        assert_eq!((size_of::<Cmsghdr<T>>() - size_of::<T>()) % size_of::<size_t>(), 0);
        Cmsghdr {
            cmsg_len: size_of::<Cmsghdr<T>>() as size_t,
            cmsg_level: level,
            cmsg_type: scm,
            __cmsg_data: data,
        }
    }
}

// The cmsg_data will be modified by recvmsg
#[allow(unused_mut)]
pub fn recvmsg<T>(sockfd: &mut AsRawFd, iov_len: usize, mut cmsg_data: T) -> io::IoResult<(ssize_t, Vec<u8>, T)> {
    let mut iov_data = Vec::with_capacity(iov_len);
    let iov_data_ptr = iov_data.as_mut_ptr();
    // The iov will be modified by recvmsg
    let mut iovv = vec!(Iovec {
        iov_base: unsafe { transmute(iov_data_ptr) },
        iov_len: iov_len as size_t,
    });
    let mut ctrl = Cmsghdr::new(SOL_SOCKET, Scm::Rights, cmsg_data);
    let mut msg = Msghdr::new(None, iovv, &mut ctrl, None);
    let size = match unsafe { raw::recvmsg(sockfd.as_raw_fd(), transmute(&mut msg), 0) } {
        -1 => return Err(io::IoError::last_error()),
        s => s,
    };
    unsafe { iov_data.set_len(msg.msg_iovlen as usize) };
    if msg.msg_controllen != size_of::<Cmsghdr<T>>() as size_t {
        // Type does not match the size + alignement
        return Err(io::standard_error(io::ShortWrite(msg.msg_controllen as usize)));
    }
    if ctrl.cmsg_len != size_of::<Cmsghdr<T>>() as size_t {
        // Bad length
        return Err(io::standard_error(io::ShortWrite(ctrl.cmsg_len as usize)));
    }
    Ok((size, iov_data, ctrl.__cmsg_data))
}

pub fn sendmsg<T>(sockfd: &mut AsRawFd, msg: Msghdr<T>) -> io::IoResult<ssize_t> {
    match unsafe { raw::sendmsg(sockfd.as_raw_fd(), transmute(&msg), 0) } {
        -1 => Err(io::IoError::last_error()),
        s => Ok(s),
    }
}
