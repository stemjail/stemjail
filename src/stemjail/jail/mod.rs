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

use self::ns::{chdir, mount, pivot_root, setgroups, umount, unshare};
use self::ns::{fs, fs0, raw, sched};
use self::libc::funcs::posix88::unistd::{fork, getgid, getuid};
use self::libc::types::os::arch::posix88::pid_t;
use std::io;
use std::io::{File, Open, Write};

#[path = "../../ffi/ns.rs" ]
mod ns;

/// Do not return error if the directory already exist
macro_rules! mkdir_if_not(
    ($path: expr) => {
        match io::fs::mkdir($path, io::UserRWX) {
            Ok(_) => {}
            Err(e) => match e.kind {
                // TODO: Fix io::PathAlreadyExists
                io::OtherIoError => {}
                _ => return Err(e)
            },
        }
    };
)

macro_rules! nested_dir(
    ($root: expr, $subdir: expr) => {
        $root.clone().join(
            match $subdir.path_relative_from(&Path::new("/")) {
                Some(p) => p,
                None => return Err(io::standard_error(io::OtherIoError)),
            }
        );
    };
)

pub struct BindMount {
    pub dst: Path,
    pub src: Path,
    pub write: bool,
}

pub struct Jail {
    name: String,
    root: Path,
    binds: Vec<BindMount>,
}

impl Jail {
    pub fn new(name: String, root: Path, binds: Vec<BindMount>) -> Jail {
        Jail {
            name: name,
            root: root,
            binds: binds,
        }
    }

    /// The current user become root
    fn init_userns(&self, pid: pid_t) -> io::IoResult<()> {
        // Do not use write/format_args_method-like macros, proc files must be
        // write only at once to avoid invalid argument.
        let uid_path = Path::new(format!("/proc/{}/uid_map", pid));
        let mut uid_file = try!(File::open_mode(&uid_path, Open, Write));
        let uid_data = format!("0 {} 1", unsafe { getuid() });
        try!(uid_file.write_str(uid_data.as_slice()));
        let gid_path = Path::new(format!("/proc/{}/gid_map", pid));
        let mut gid_file = try!(File::open_mode(&gid_path, Open, Write));
        let gid_data = format!("0 {} 1", unsafe { getgid() });
        try!(gid_file.write_str(gid_data.as_slice()));
        Ok(())
    }

    fn add_bind(&self, bind: &BindMount) -> io::IoResult<()> {
        // FIXME: There must be no submount (maybe fs_fully_visible check?)
        let dst = nested_dir!(self.root, bind.dst);
        let flags = fs::MsMgcVal | fs::MsBind;
        mkdir_if_not!(&dst);
        try!(mount(&bind.src, &dst, &"none".to_string(),
                    &flags, &None));
        if ! bind.write {
            let flags = flags | fs::MsRemount | fs::MsRdonly;
            try!(mount(&bind.src, &dst, &"none".to_string(), &flags, &None));
        }
        Ok(())
    }

    // TODO: impl Drop to unmount
    fn init_fs(&self) -> io::IoResult<()> {
        // Prepare to remove all parent mounts with a pivot
        let root_flags = fs::MsMgcVal | fs::MsBind;
        try!(mount(&self.root, &self.root, &"none".to_string(),
                    &root_flags, &None));
        try!(chdir(&self.root));

        let proc_src = Path::new("proc");
        let proc_dst = self.root.clone().join(proc_src.clone());
        mkdir_if_not!(&proc_dst);
        try!(mount(&proc_src, &proc_dst, &"proc".to_string(),
                    &fs::MsMgcVal, &None));

        for bind in self.binds.iter() {
            try!(self.add_bind(bind));
        }

        // Finalize the pivot
        let old_root = Path::new("old_root");
        try!(io::fs::mkdir(&old_root, io::UserRWX));
        try!(pivot_root(&self.root, &old_root));

        // Cleanup parent mounts
        try!(umount(&old_root, &fs0::MntDetach));
        try!(io::fs::rmdir(&old_root));
        Ok(())
    }

    pub fn run(&mut self, run: Path) {
        println!("Running jail {}", self.name);

        // TODO: Replace fork with a new process creation and dedicated protocol
        // Fork a new process
        let mut sync_parent = match io::pipe::PipeStream::pair() {
            Ok(p) => p,
            Err(e) => fail!("Fail to fork: {}", e),
        };
        let mut sync_child = match io::pipe::PipeStream::pair() {
            Ok(p) => p,
            Err(e) => fail!("Fail to fork: {}", e),
        };
        let pid = unsafe { fork() };
        if pid < 0 {
            fail!("Fail to fork");
        } else if pid == 0 {
            // Child
            println!("Child jailing into {}", self.root.display());
            match unshare(
                    sched::CloneNewipc |
                    sched::CloneNewnet |
                    sched::CloneNewns |
                    sched::CloneNewpid |
                    sched::CloneNewuser |
                    sched::CloneNewuts
            ) {
                Ok(_) => {},
                Err(e) => fail!("Fail to unshare: {}", e),
            }

            // Sync with parent
            match sync_parent.writer.write_i8(0) {
                Ok(_) => {}
                Err(e) => fail!("Fail to synchronise with parent: {}", e),
            }
            let _ = sync_child.reader.read_i8();

            // Need to fork to be able to mount /proc
            let pid = unsafe { fork() };
            if pid != 0 {
                // Parent
                let mut status: libc::c_int = 0;
                let _ = unsafe { raw::waitpid(pid, &mut status, 0) };
                return;
            } else {
                let groups = Vec::new();
                match setgroups(groups) {
                    Ok(_) => {}
                    Err(e) => fail!("Fail to set groups: {}", e),
                }
                match unsafe { getuid() } {
                    0 => {}
                    _ => fail!("Fail to got root"),
                }
                match self.init_fs() {
                    Ok(_) => {}
                    Err(e) => fail!("Fail to initialize the file system: {}", e),
                }

                match io::Command::new(&run)
                        .stdin(io::process::InheritFd(libc::STDIN_FILENO))
                        .stdout(io::process::InheritFd(libc::STDOUT_FILENO))
                        .stderr(io::process::InheritFd(libc::STDERR_FILENO))
                        .spawn() {
                    Ok(_) => {},
                    Err(e) => fail!("Fail to execute process: {}", e),
                }
                return;
            }
        } else {
            // Parent
            // TODO: Send fail command to the child if any error
            let _ = sync_parent.reader.read_i8();
            match self.init_userns(pid) {
                Ok(_) => {}
                Err(e) => fail!("Fail to initialize user namespace: {}", e),
            }
            match sync_child.writer.write_i8(0) {
                Ok(_) => {}
                Err(e) => fail!("Fail to synchronise with child: {}", e),
            }
            println!("Waiting for child {} to terminate", pid);
            let mut status: libc::c_int = 0;
            match unsafe { raw::waitpid(pid, &mut status, 0) } {
                -1 => fail!("Fail to wait for child {}", pid),
                _ => {}
            }
        }
    }
}
