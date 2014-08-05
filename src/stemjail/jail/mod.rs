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

use self::fsb::dup;
use self::ns::{chdir, mount, pivot_root, setgroups, umount, unshare};
use self::ns::{fs, fs0, raw, sched};
use self::libc::funcs::posix88::unistd::{fork, setsid, getgid, getuid};
use self::libc::types::os::arch::posix88::pid_t;
use self::native::io::file::FileDesc;
use std::io;
use std::io::{File, Open, Write};

#[path = "../../ffi/fs.rs" ]
mod fsb;
#[path = "../../ffi/ns.rs" ]
mod ns;

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

/// Do not return error if the directory already exist
fn mkdir_if_not(path: &Path) -> io::IoResult<()> {
    match io::fs::mkdir_recursive(path, io::UserRWX) {
        Ok(_) => Ok(()),
        Err(e) => match e.kind {
            // TODO: Fix io::PathAlreadyExists
            io::OtherIoError => Ok(()),
            _ => Err(e)
        },
    }
}

/// Do not return error if the file already exist
fn touch_if_not(path: &Path) -> io::IoResult<()> {
    match path.stat() {
        Ok(fs) => match fs.kind {
            io::TypeDirectory =>
                Err(io::standard_error(io::MismatchedFileTypeForOperation)),
            _ => Ok(()),
        },
        Err(e) => match e.kind {
            io::FileNotFound => match io::fs::File::create(path) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            },
            _ => Err(e),
        },
    }
}

/// Create a dst file or directory according to the src
fn create_same_type(src: &Path, dst: &Path) -> io::IoResult<()> {
    match try!(src.stat()).kind {
        io::TypeDirectory => {
            try!(mkdir_if_not(dst));
        }
        _ => {
            let mut d = dst.clone();
            d.pop();
            try!(mkdir_if_not(&d));
            try!(touch_if_not(dst));
        }
    }
    Ok(())
}

pub struct Stdio {
    pub stdin: FileDesc,
    pub stdout: FileDesc,
    pub stderr: FileDesc,
}

impl Stdio {
    pub fn new(fd: FileDesc) -> io::IoResult<Stdio> {
        Ok(Stdio {
            // Can't close on drop because of the io::Command FD auto-closing
            stdin: try!(dup(&fd, false)),
            stdout: try!(dup(&fd, false)),
            stderr: try!(dup(&fd, false)),
        })
    }
}

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

    fn init_dev(&self, devdir: &Path) -> io::IoResult<()> {
        info!("Populating /dev");
        let devdir_full = nested_dir!(self.root, devdir);
        try!(mkdir_if_not(&devdir_full));
        let name = Path::new("dev");
        let flags = fs::MsMgcVal;
        try!(mount(&name, &devdir_full, &"tmpfs".to_string(), &flags, &None));

        // Create mount points
        let devs = &[
            "null",
            "zero",
            "full",
            "urandom",
            ];
        for dev in devs.iter() {
            let src = Path::new(format!("/dev/{}", dev));
            let dst = devdir_full.clone().join(Path::new(*dev));
            try!(create_same_type(&src, &dst));
        }
        let links = &[
            ("fd", "/proc/self/fd"),
            ("random", "urandom")
            ];
        for &(src, dst) in links.iter() {
            let src = devdir_full.clone().join(Path::new(src));
            let dst = Path::new(dst);
            try!(io::fs::symlink(&dst, &src));
        }

        // Seal /dev
        // TODO: Drop the root user to realy seal something…
        let bind = BindMount { src: devdir_full.clone(), dst: devdir.clone(), write: false };
        try!(self.add_bind(&bind));

        for dev in devs.iter() {
            let src = Path::new(format!("/dev/{}", dev));
            let bind = BindMount { src: src.clone(), dst: src, write: true };
            try!(self.add_bind(&bind));
        }
        Ok(())
    }

    fn add_bind(&self, bind: &BindMount) -> io::IoResult<()> {
        // FIXME: There must be no submount (maybe fs_fully_visible check?)
        info!("Bind mounting {}", bind.dst.display());
        let dst = nested_dir!(self.root, bind.dst);
        let flags = fs::MsMgcVal | fs::MsBind;

        // Create needed directorie(s) and/or file
        try!(create_same_type(&bind.src, &dst));

        try!(mount(&bind.src, &dst, &"none".to_string(), &flags, &None));
        if ! bind.write {
            let flags = flags | fs::MsRemount | fs::MsRdonly;
            try!(mount(&bind.src, &dst, &"none".to_string(), &flags, &None));
        }
        Ok(())
    }

    // TODO: impl Drop to unmount and remove mount directories/files
    fn init_fs(&self) -> io::IoResult<()> {
        // Prepare to remove all parent mounts with a pivot
        let root_flags = fs::MsMgcVal | fs::MsBind;
        try!(mount(&self.root, &self.root, &"none".to_string(),
                    &root_flags, &None));
        try!(chdir(&self.root));

        // procfs
        let proc_src = Path::new("proc");
        let proc_dst = self.root.clone().join(proc_src.clone());
        try!(mkdir_if_not(&proc_dst));
        try!(mount(&proc_src, &proc_dst, &"proc".to_string(),
                    &fs::MsMgcVal, &None));

        // Devices
        try!(self.init_dev(&Path::new("/dev")));

        for bind in self.binds.iter() {
            try!(self.add_bind(bind));
        }

        // Finalize the pivot
        let old_root = Path::new("tmp");
        try!(mkdir_if_not(&old_root));
        try!(pivot_root(&self.root, &old_root));

        // Cleanup parent mounts
        try!(umount(&old_root, &fs0::MntDetach));
        Ok(())
    }

    pub fn run(&mut self, run: Path, stdio: Option<Stdio>) {
        info!("Running jail {}", self.name);

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
            fail!("Fail to fork #1");
        } else if pid == 0 {
            // Child
            info!("Child jailing into {}", self.root.display());
            // Become a process group leader
            // TODO: Change behavior for dedicated TTY
            match unsafe { setsid() } {
                -1 => fail!("Fail setsid: {}", io::IoError::last_error()),
                _ => {}
            }
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

            // Need to fork because of the PID namespace and the group ID
            let pid = unsafe { fork() };
            if pid < 0 {
                fail!("Fail to fork #2");
            } else if pid == 0 {
                // Child
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

                let (stdin, stdout, stderr) = match stdio {
                    Some(s) => {(
                        io::process::InheritFd(s.stdin.fd()),
                        io::process::InheritFd(s.stdout.fd()),
                        io::process::InheritFd(s.stderr.fd()),
                    )},
                    None => {(
                        io::process::InheritFd(libc::STDIN_FILENO),
                        io::process::InheritFd(libc::STDOUT_FILENO),
                        io::process::InheritFd(libc::STDERR_FILENO),
                    )}
                };
                // FIXME when using env* functions: task '<unnamed>' failed at 'could not initialize task_rng: couldn't open file (no such file or directory (No such file or directory); path=/dev/urandom; mode=open; access=read)', .../rust/src/libstd/rand/mod.rs:200
                let env: Vec<(String, String)> = Vec::with_capacity(0);
                match io::Command::new(&run)
                        .stdin(stdin)
                        .stdout(stdout)
                        .stderr(stderr)
                        .env_set_all(env.as_slice())
                        .spawn() {
                    Ok(_) => {},
                    Err(e) => fail!("Fail to execute process: {}", e),
                }
                return;
            } else {
                // Parent
                let mut status: libc::c_int = 0;
                let _ = unsafe { raw::waitpid(pid, &mut status, 0) };
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
            debug!("Waiting for child {} to terminate", pid);
            let mut status: libc::c_int = 0;
            match unsafe { raw::waitpid(pid, &mut status, 0) } {
                -1 => fail!("Fail to wait for child {}", pid),
                _ => {}
            }
        }
    }
}
