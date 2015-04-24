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

use std::env;
use std::ffi::CString;
use std::io;
use std::path::Path;
use std::ptr;

#[path = "gen/sched.rs"]
pub mod sched;
#[path = "gen/fs.rs"]
pub mod fs;
#[path = "gen/fs0.rs"]
pub mod fs0;

pub mod raw {
    extern crate libc;

    use self::libc::{c_char, c_int, c_uint, c_ulong, pid_t};

    extern {
        pub fn chroot(path: *const c_char) -> c_int;
        pub fn mount(source: *const c_char, target: *const c_char,
                     filesystemtype: *const c_char, mountflags: c_ulong,
                     data: *const c_char) -> c_int;
        pub fn pivot_root(new_root: *const c_char, put_old: *const c_char) -> c_int;
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
    pub fn gettid() -> pid_t {
        unsafe { sc0::syscall(186) as pid_t }
    }
}

// TODO: use the sys::cvt familly

#[allow(dead_code)]
pub fn chroot<T>(path: T) -> io::Result<()> where T: AsRef<Path> {
    let path = path.as_ref();
    try!(env::set_current_dir(path));
    let path = try!(CString::new(path2bytes!(&path)));
    match unsafe { raw::chroot(path.as_ptr()) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

pub fn mount<T,U>(source: T, target: U, filesystemtype: &str, mountflags: &fs::MsFlags,
                data: &Option<&str>) -> io::Result<()> where T: AsRef<Path>, U: AsRef<Path> {
    let src = try!(CString::new(path2bytes!(&source)));
    let tgt = try!(CString::new(path2bytes!(&target)));
    let fst = try!(CString::new(filesystemtype.as_bytes()));
    let ret = match data {
        &Some(ref data) => {
            let opt = try!(CString::new(data.as_bytes()));
            unsafe { raw::mount(src.as_ptr(), tgt.as_ptr(), fst.as_ptr(), mountflags.bits(), opt.as_ptr()) }
        },
        &None => unsafe {
            raw::mount(src.as_ptr(), tgt.as_ptr(), fst.as_ptr(), mountflags.bits(), ptr::null())
        }
    };
    match ret {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

pub fn pivot_root<T,U>(new_root: T, put_old: U) -> io::Result<()>
        where T: AsRef<Path>, U: AsRef<Path> {
    let new_root = try!(CString::new(path2bytes!(&new_root)));
    let put_old = try!(CString::new(path2bytes!(&put_old)));
    match unsafe { raw::pivot_root(new_root.as_ptr(), put_old.as_ptr()) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

#[allow(dead_code)]
pub fn umount<T>(target: T, flags: &fs0::MntFlags) -> io::Result<()> where T: AsRef<Path> {
    let target = try!(CString::new(path2bytes!(&target)));
    match unsafe { raw::umount2(target.as_ptr(), flags.bits()) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

pub fn unshare(flags: sched::CloneFlags) -> io::Result<()> {
    match unsafe { raw::unshare(flags.bits()) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}
