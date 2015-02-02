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

use self::libc::size_t;
use self::libc::types::os::arch::posix88::gid_t;
use std::ffi::CString;
use std::old_io as io;
use std::os;
use std::ptr;

#[path = "gen/sched.rs"]
pub mod sched;
#[path = "gen/fs.rs"]
pub mod fs;
#[path = "gen/fs0.rs"]
pub mod fs0;

pub mod raw {
    extern crate libc;

    use self::libc::{c_char, c_int, size_t, c_uint, c_ulong};
    use self::libc::types::os::arch::posix88::{gid_t, pid_t};

    extern {
        pub fn chroot(path: *const c_char) -> c_int;
        pub fn mount(source: *const c_char, target: *const c_char,
                     filesystemtype: *const c_char, mountflags: c_ulong,
                     data: *const c_char) -> c_int;
        pub fn pivot_root(new_root: *const c_char, put_old: *const c_char) -> c_int;
        pub fn setgroups(size: size_t, list: *const gid_t) -> c_int;
        pub fn umount2(target: *const c_char, flags: c_uint) -> c_int;
        pub fn unshare(flags: c_uint) -> c_int;
        pub fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t;
    }

    // Syscall without argument
    mod sc0 {
        use super::libc::c_int;

        extern {
            pub fn syscall(number: c_int) -> c_int;
        }
    }

    // Syscall numbers from x86_64-linux-gnu/asm/unistd_64.h
    #[cfg(target_arch="x86_64")]
    #[allow(dead_code)]
    pub fn gettid() -> pid_t {
        unsafe { sc0::syscall(186) as pid_t }
    }
}

#[allow(dead_code)]
pub fn chroot(path: &Path) -> io::IoResult<()> {
    try!(os::change_dir(path));
    let path = CString::from_slice(path.as_vec());
    match unsafe { raw::chroot(path.as_ptr()) } {
        0 => Ok(()),
        _ => Err(io::IoError::last_error()),
    }
}

pub fn mount(source: &Path, target: &Path, filesystemtype: &str,
             mountflags: &fs::MsFlags, data: &Option<&str>) -> io::IoResult<()> {
    let src = CString::from_slice(source.as_vec());
    let tgt = CString::from_slice(target.as_vec());
    let fst = CString::from_slice(filesystemtype.as_bytes());
    let ret = match data {
        &Some(ref data) => {
            let opt = CString::from_slice(data.as_bytes());
            unsafe { raw::mount(src.as_ptr(), tgt.as_ptr(), fst.as_ptr(), mountflags.bits(), opt.as_ptr()) }
        },
        &None => unsafe {
            raw::mount(src.as_ptr(), tgt.as_ptr(), fst.as_ptr(), mountflags.bits(), ptr::null())
        }
    };
    match ret {
        0 => Ok(()),
        _ => Err(io::IoError::last_error()),
    }
}

pub fn pivot_root(new_root: &Path, put_old: &Path) -> io::IoResult<()> {
    let new_root = CString::from_slice(new_root.as_vec());
    let put_old = CString::from_slice(put_old.as_vec());
    match unsafe { raw::pivot_root(new_root.as_ptr(), put_old.as_ptr()) } {
        0 => Ok(()),
        _ => Err(io::IoError::last_error()),
    }
}

pub fn setgroups(groups: Vec<gid_t>) -> io::IoResult<()> {
    match unsafe { raw::setgroups(groups.len() as size_t, groups.as_ptr()) } {
        -1 => Err(io::IoError::last_error()),
        _ => Ok(()),
    }
}

pub fn umount(target: &Path, flags: &fs0::MntFlags) -> io::IoResult<()> {
    let target = CString::from_slice(target.as_vec());
    match unsafe { raw::umount2(target.as_ptr(), flags.bits()) } {
        0 => Ok(()),
        _ => Err(io::IoError::last_error()),
    }
}

pub fn unshare(flags: sched::CloneFlags) -> io::IoResult<()> {
    match unsafe { raw::unshare(flags.bits()) } {
        0 => Ok(()),
        _ => Err(io::IoError::last_error()),
    }
}
