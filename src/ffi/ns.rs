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
use std::{io, os};
use std::ptr;

#[path = "gen/sched.rs"]
mod sched;
#[path = "gen/fs.rs"]
mod fs;
#[path = "gen/fs0.rs"]
mod fs0;

mod raw {
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

#[allow(dead_code)]
pub fn mount(source: &Path, target: &Path, filesystemtype: &String,
             mountflags: &fs::MsFlags, data: &Option<String>) -> io::IoResult<()> {
    let src = path2str!(source);
    let tgt = path2str!(target);
    src.with_c_str(|src| {
        tgt.with_c_str(|tgt| {
            filesystemtype.with_c_str(|fst| {
                let ret = match data {
                    &Some(ref data) => data.with_c_str(|opt| { unsafe {
                        raw::mount(src, tgt, fst, mountflags.bits(), opt)
                    } }),
                    &None => unsafe {
                        raw::mount(src, tgt, fst, mountflags.bits(), ptr::null())
                    }
                };
                match ret {
                    0 => Ok(()),
                    _ => Err(io::IoError::last_error()),
                }
            })
        })
    })
}

#[allow(dead_code)]
pub fn pivot_root(new_root: &Path, put_old: &Path) -> io::IoResult<()> {
    let new_root = path2str!(new_root);
    let put_old = path2str!(put_old);
    new_root.with_c_str(|new_root| {
        put_old.with_c_str(|put_old| {
            match unsafe { raw::pivot_root(new_root, put_old) } {
                0 => Ok(()),
                _ => Err(io::IoError::last_error()),
            }
        })
    })
}

#[allow(dead_code)]
pub fn setgroups(groups: Vec<gid_t>) -> io::IoResult<()> {
    match unsafe { raw::setgroups(groups.len() as size_t, groups.as_ptr()) } {
        -1 => Err(io::IoError::last_error()),
        _ => Ok(()),
    }
}

#[allow(dead_code)]
pub fn umount(target: &Path, flags: &fs0::MntFlags) -> io::IoResult<()> {
    let target = path2str!(target);
    target.with_c_str(|target| {
        match unsafe { raw::umount2(target, flags.bits()) } {
            0 => Ok(()),
            _ => Err(io::IoError::last_error()),
        }
    })
}

#[allow(dead_code)]
pub fn unshare(flags: sched::CloneFlags) -> io::IoResult<()> {
    match unsafe { raw::unshare(flags.bits()) } {
        0 => Ok(()),
        _ => Err(io::IoError::last_error()),
    }
}
