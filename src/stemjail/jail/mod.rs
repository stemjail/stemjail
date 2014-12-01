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

use self::libc::funcs::posix88::unistd::{fork, setsid, getgid, getuid};
use self::libc::types::os::arch::posix88::pid_t;
use self::ns::{fs, fs0, raw, sched};
use self::ns::{mount, pivot_root, setgroups, umount, unshare};
use std::io;
use std::io::{File, Open, Write};
use std::io::fs::PathExtensions;
use std::os::change_dir;
use std::sync::{Arc, RWLock};

pub use self::session::Stdio;

#[path = "../../ffi/fs.rs" ]
mod fsb;
#[path = "../../ffi/ns.rs" ]
mod ns;
mod session;

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
    match io::fs::mkdir_recursive(path, io::USER_RWX) {
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

pub struct BindMount {
    pub dst: Path,
    pub src: Path,
    pub write: bool,
}

// TODO: Add UUID
pub struct Jail {
    name: String,
    root_src: Path,
    root_dst: Path,
    binds: Vec<BindMount>,
    stdio: Option<Stdio>,
    pid: Arc<RWLock<Option<pid_t>>>,
    end_event: Option<Receiver<Result<(), ()>>>,
}

impl Jail {
    pub fn new(name: String, root: Path, binds: Vec<BindMount>) -> Jail {
        Jail {
            name: name,
            root_src: root,
            // TODO: Add a fallback for root_dst
            root_dst: Path::new("/proc/self/fdinfo"),
            binds: binds,
            stdio: None,
            pid: Arc::new(RWLock::new(None)),
            end_event: None,
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
        let devdir_full = nested_dir!(self.root_dst, devdir);
        try!(mkdir_if_not(&devdir_full));
        let name = Path::new("dev");
        let dev_flags = fs::MsFlags::empty();
        try!(mount(&name, &devdir_full, &"tmpfs".to_string(), &dev_flags, &None));

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
        let dst = nested_dir!(self.root_dst, bind.dst);
        let bind_flags = fs::MS_BIND | fs::MS_REC;

        // Create needed directorie(s) and/or file
        try!(create_same_type(&bind.src, &dst));

        try!(mount(&bind.src, &dst, &"none".to_string(), &bind_flags, &None));
        if ! bind.write {
            let bind_flags = bind_flags | fs::MS_REMOUNT | fs::MS_RDONLY;
            try!(mount(&bind.src, &dst, &"none".to_string(), &bind_flags, &None));
        }
        Ok(())
    }

    // TODO: impl Drop to unmount and remove mount directories/files
    fn init_fs(&self) -> io::IoResult<()> {
        // Prepare to remove all parent mounts with a pivot
        // TODO: Add a path blacklist to hide some directories (e.g. when root_src == /)
        let root_flags = fs::MS_BIND | fs::MS_REC;
        try!(mount(&self.root_src, &self.root_dst, &"none".to_string(), &root_flags, &None));
        try!(change_dir(&self.root_dst));

        // procfs
        let proc_src = Path::new("proc");
        let proc_dst = self.root_dst.clone().join(proc_src.clone());
        try!(mkdir_if_not(&proc_dst));
        let proc_flags = fs::MsFlags::empty();
        try!(mount(&proc_src, &proc_dst, &"proc".to_string(), &proc_flags, &None));

        // Devices
        try!(self.init_dev(&Path::new("/dev")));

        for bind in self.binds.iter() {
            try!(self.add_bind(bind));
        }

        // Finalize the pivot
        let old_root = Path::new("tmp");
        try!(mkdir_if_not(&old_root));
        try!(pivot_root(&self.root_dst, &old_root));

        // Cleanup parent mounts
        try!(umount(&old_root, &fs0::MNT_DETACH));
        Ok(())
    }

    // TODO: Return IoResult<()>
    pub fn run(&mut self, run: &Path, args: &Vec<String>, stdio: Option<Stdio>) {
        info!("Running jail {}", self.name);

        // TODO: Replace fork with a new process creation and dedicated protocol
        // Fork a new process
        let mut sync_parent = match io::pipe::PipeStream::pair() {
            Ok(p) => p,
            Err(e) => panic!("Fail to create pipe #1: {}", e),
        };
        let mut sync_child = match io::pipe::PipeStream::pair() {
            Ok(p) => p,
            Err(e) => panic!("Fail to create pipe #2: {}", e),
        };
        let (mut jail_pid_rx, mut jail_pid_tx) = match io::pipe::PipeStream::pair() {
            Ok(p) => (p.reader, p.writer),
            Err(e) => panic!("Fail to create pipe #3: {}", e),
        };

        let (mut slave_fd, stdin, stdout, stderr) = match stdio {
            // TODO: Use pipes if no TTY
            Some(mut s) => {
                // XXX: The TTY must be new
                let slave_fd = s.take_slave_fd().unwrap();
                let fd = slave_fd.fd();
                //pty::set_nonblock(&fd);
                self.stdio = Some(s);
                // Keep the slave FD open until the spawn
                (
                    Some(slave_fd),
                    io::process::InheritFd(fd),
                    io::process::InheritFd(fd),
                    io::process::InheritFd(fd),
                )
            },
            None => {(
                None,
                io::process::InheritFd(libc::STDIN_FILENO),
                io::process::InheritFd(libc::STDOUT_FILENO),
                io::process::InheritFd(libc::STDERR_FILENO),
            )}
        };
        let (end_tx, end_rx) = channel();
        //let (end_rx, end_tx): (Receiver<()>, Sender<()>)= channel();
        self.end_event = Some(end_rx);
        let jail_pid = self.pid.clone();

        // Dedicated task to wait for the jail process end
        // TODO: Use Rust (synchronised) task wrapping fork to get free Rust extra checks
        let pid = unsafe { fork() };
        if pid < 0 {
            panic!("Fail to fork #1");
        } else if pid == 0 {
            // Child
            drop(jail_pid_rx);
            info!("Child jailing into {}", self.root_src.display());
            // Become a process group leader
            // TODO: Change behavior for dedicated TTY
            match unsafe { setsid() } {
                -1 => panic!("Fail setsid: {}", io::IoError::last_error()),
                _ => {}
            }
            match unshare(
                    sched::CLONE_NEWIPC |
                    sched::CLONE_NEWNET |
                    sched::CLONE_NEWNS |
                    sched::CLONE_NEWPID |
                    sched::CLONE_NEWUSER |
                    sched::CLONE_NEWUTS
            ) {
                Ok(_) => {},
                Err(e) => panic!("Fail to unshare: {}", e),
            }

            // Sync with parent
            match sync_parent.writer.write_i8(0) {
                Ok(_) => {}
                Err(e) => panic!("Fail to synchronise with parent #1: {}", e),
            }
            match sync_child.reader.read_i8() {
                Ok(_) => {}
                Err(e) => panic!("Fail to synchronise with parent #2: {}", e),
            }

            // Need to fork because of the PID namespace and the group ID
            let pid = unsafe { fork() };
            if pid < 0 {
                panic!("Fail to fork #2");
            } else if pid == 0 {
                // Child
                let groups = Vec::new();
                match setgroups(groups) {
                    Ok(_) => {}
                    Err(e) => panic!("Fail to set groups: {}", e),
                }
                match unsafe { getuid() } {
                    0 => {}
                    _ => panic!("Fail to got root"),
                }
                // TODO: Expose the TTY
                match self.init_fs() {
                    Ok(_) => {}
                    Err(e) => panic!("Fail to initialize the file system: {}", e),
                }

                // FIXME when using env* functions: task '<unnamed>' failed at 'could not initialize task_rng: couldn't open file (no such file or directory (No such file or directory); path=/dev/urandom; mode=open; access=read)', .../rust/src/libstd/rand/mod.rs:200
                let env: Vec<(String, String)> = Vec::with_capacity(0);
                // TODO: Try using detached()
                let mut process = match io::Command::new(run)
                        .stdin(stdin)
                        .stdout(stdout)
                        .stderr(stderr)
                        .env_set_all(env.as_slice())
                        .args(args.as_slice())
                        .spawn() {
                    Ok(p) => p,
                    Err(e) => panic!("Fail to execute process: {}", e),
                };
                // Need to keep the slave TTY open until passing to the child
                drop(slave_fd.take());
                // TODO: Check 32-bits compatibility with other arch
                match jail_pid_tx.write_le_i32(process.id()) {
                    Ok(_) => {}
                    Err(e) => panic!("Fail to send child PID: {}", e),
                }
                drop(jail_pid_tx);
                // TODO: Forward the ProcessExit to the jail object
                let ret = process.wait();
                debug!("Jail process exit: {}", ret);
                unsafe { libc::exit(0); }
            } else {
                // Parent
                drop(jail_pid_tx);
                drop(slave_fd.take());
                let mut status: libc::c_int = 0;
                // TODO: Replace waitpid(2) with wait(2)
                let _ = unsafe { raw::waitpid(pid, &mut status, 0) };
                unsafe { libc::exit(0); }
            }
        } else {
            // Parent
            drop(jail_pid_tx);
            drop(slave_fd.take());
            // TODO: Send fail command to the child if any error
            let _ = sync_parent.reader.read_i8();
            match self.init_userns(pid) {
                Ok(_) => {}
                Err(e) => panic!("Fail to initialize user namespace: {}", e),
            }
            match sync_child.writer.write_i8(0) {
                Ok(_) => {}
                Err(e) => panic!("Fail to synchronise with child: {}", e),
            }
            // Get the child PID
            match jail_pid_rx.read_le_i32() {
                Ok(p) => {
                    let mut lock = jail_pid.write();
                    *lock = Some(p);
                }
                Err(e) => panic!("Fail to get jail PID: {}", e),
            }
            debug!("Got jail PID: {}", *jail_pid.read());
            debug!("Waiting for child {} to terminate", pid);
            spawn(proc() {
                let mut status: libc::c_int = 0;
                // TODO: Replace waitpid(2) with wait(2)
                match unsafe { raw::waitpid(pid, &mut status, 0) } {
                    //-1 => panic!("Fail to wait for child {}", pid),
                    -1 => drop(end_tx.send_opt(Err(()))),
                    _ => { {
                            let mut lock = jail_pid.write();
                            *lock = None;
                        }
                        drop(end_tx.send_opt(Ok(())));
                    }
                }
            });
        }
    }

    pub fn get_stdio(&self) -> &Option<Stdio> {
        &self.stdio
    }

    pub fn wait(&self) -> Result<(), ()> {
        match &self.end_event {
            &Some(ref event) =>  match event.recv_opt() {
                Ok(_) => Ok(()),
                Err(_) => Err(()),
            },
            &None => Err(()),
        }
    }
}
