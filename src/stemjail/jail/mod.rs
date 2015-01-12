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

extern crate mnt;
extern crate libc;

use self::libc::funcs::posix88::unistd::{fork, setsid, getgid, getuid};
use self::libc::types::os::arch::posix88::pid_t;
use self::mount::Mount;
use self::ns::{fs, fs0, raw, sched};
use self::ns::{mount, pivot_root, setgroups, umount, unshare};
use std::borrow::{Borrowed, Owned};
use std::io;
use std::io::{File, Open, Write};
use std::io::fs::PathExtensions;
use std::os::{change_dir, env};
use std::os::unix::AsRawFd;
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

// TODO: Add tmpfs prelude to not pollute the root

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

#[deriving(Clone)]
pub struct BindMount {
    pub src: Path,
    pub dst: Path,
    pub write: bool,
}

#[deriving(Clone)]
pub struct TmpfsMount<'a> {
    pub name: Option<&'a str>,
    pub dst: Path,
}

// TODO: Add UUID
pub struct Jail<'a> {
    name: String,
    root: BindMount,
    binds: Vec<BindMount>,
    tmps: Vec<TmpfsMount<'a>>,
    stdio: Option<Stdio>,
    pid: Arc<RWLock<Option<pid_t>>>,
    end_event: Option<Receiver<Result<(), ()>>>,
}

impl<'a> Jail<'a> {
    // TODO: Check configuration for duplicate binds entries and refuse to use it if so
    pub fn new(name: String, root: Path, binds: Vec<BindMount>, tmps: Vec<TmpfsMount>) -> Jail {
        // TODO: Check for a real procfs
        let tmp_dir = Path::new(format!("/proc/{}/fdinfo", unsafe { libc::getpid() }));
        // Hack to cleanly manage the root bind mount
        let mut root_binds = vec!( BindMount { src: root.clone(), dst: Path::new("/"), write: false } );
        root_binds.push_all(binds.as_slice());
        //let root_binds = binds;
        Jail {
            name: name,
            // TODO: Add a fallback for root.dst
            root: BindMount { src: root, dst: tmp_dir, write: false },
            binds: root_binds,
            tmps: tmps,
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
        info!("Populating {}", devdir.display());
        let devdir_full = nested_dir!(self.root.dst, devdir);
        try!(mkdir_if_not(&devdir_full));
        try!(self.add_tmpfs(&TmpfsMount { name: Some("dev"), dst: devdir.clone() }));

        // TODO: Use macro
        // Create mount points
        let devs = &[
            "null",
            "zero",
            "full",
            "urandom",
            ];
        let mut devs: Vec<BindMount> = devs.iter().map(|dev| {
            let src = devdir.clone().join(&Path::new(*dev));
            BindMount { src: src.clone(), dst: src, write: true }
        }).collect();

        // Add current TTY
        // TODO: Add dynamic TTY list reload
        match self.stdio {
            Some(ref s) => match s.get_path() {
                    // FIXME: Assume `p` begin with "/dev/"
                    Some(p) => devs.push(BindMount { src: p.clone(), dst: p.clone(), write: true }),
                    None => {}
                },
            None => {}
        }

        for dev in devs.iter() {
            debug!("Creating {}", dev.dst.display());
            let dst = nested_dir!(self.root.dst, dev.dst);
            try!(create_same_type(&dev.src, &dst));
            let bind = BindMount { src: dev.src.clone(), dst: dst.clone(), write: true };
            try!(self.add_bind(&bind, true));
        }
        let links = &[
            ("fd", "/proc/self/fd"),
            ("random", "urandom")
            ];
        for &(dst, src) in links.iter() {
            let src = Path::new(src);
            let dst = devdir_full.clone().join(&dst);
            try!(io::fs::symlink(&src, &dst));
        }

        // Seal /dev
        // TODO: Drop the root user to realy seal something…
        let dev_flags = fs::MS_BIND | fs::MS_REMOUNT | fs::MS_RDONLY;
        try!(mount(&Path::new("none"), &devdir_full, "", &dev_flags, &None));

        Ok(())
    }

    fn add_bind(&self, bind: &BindMount, is_absolute: bool) -> io::IoResult<()> {
        let dst = if is_absolute {
            Borrowed(&bind.dst)
        } else {
            Owned(nested_dir!(self.root.dst, bind.dst))
        };
        let dst = dst.deref();
        debug!("Bind mounting {}", bind.src.display());

        // Create needed directorie(s) and/or file
        try!(create_same_type(&bind.src, dst));

        let none_str = "none";
        // The fs/namespace.c:clone_mnt kernel function forbid unprivileged users (i.e.
        // CL_UNPRIVILEGED) to reveal what is under a mount, so we need to recursively bind mount.
        let bind_flags = fs::MS_BIND | fs::MS_REC;
        try!(mount(&bind.src, dst, none_str, &bind_flags, &None));
        if ! bind.write {
            // When write action is forbiden we must not use the MS_REC to avoid unattended
            // read/write files during the jail life.
            let none_path = Path::new("none");
            // Seal the vfsmount: good to not receive new mounts but block unmount as well
            // TODO: Add a "unshare <path>" command to remove a to-be-unmounted path
            let bind_flags = fs::MS_PRIVATE | fs::MS_REC;
            try!(mount(&none_path, dst, none_str, &bind_flags, &None));
            // Remount read-only
            let bind_flags = fs::MS_BIND | fs::MS_REMOUNT | fs::MS_RDONLY;
            try!(mount(&none_path, dst, none_str, &bind_flags, &None));
        }
        Ok(())
    }

    fn expand_binds(&self) -> io::IoResult<Vec<BindMount>> {
        let host_mounts = {
            match Mount::get_mounts(&Path::new("/")) {
                Ok(list) => {
                    let mut ret = vec!();
                    let devdir = Path::new("/dev");
                    let procdir = Path::new("/proc");
                    for mount in Mount::remove_overlaps(list).into_iter() {
                        if ! self.root.dst.is_ancestor_of(&mount.file)
                                && ! devdir.is_ancestor_of(&mount.file)
                                && ! procdir.is_ancestor_of(&mount.file) {
                            ret.push(mount);
                        }
                    }
                    ret
                },
                Err(e) => {
                    // TODO: Add FromError impl to IoResult
                    debug!("Error: get_mounts: {}", e);
                    return Err(io::standard_error(io::OtherIoError));
                }
            }
        };
        // Need to keep the mount points order and prioritize the last (i.e. user) mount points
        let mut all_binds = vec!();
        // TODO: Add a black list with /dev and /proc
        for bind in self.binds.iter() {
            // TODO: Check to not replace the root.dst mount points
            // FIXME: Extend the full path (like "readlink -f") to not recursively mount
            // Complete with all child mount points if needed (i.e. read-only mount tree)
            let bind = bind.clone();
            if bind.write {
                all_binds.push(bind);
            } else {
                let bind_ref = bind.clone();
                let mut sub_binds = vec!(bind);
                for mount in host_mounts.iter() {
                    if bind_ref.src.is_ancestor_of(&mount.file) && bind_ref.src != mount.file {
                        let file = mount.file.clone();
                        let new_bind = BindMount { src: file.clone(), dst: file, write: false };
                        sub_binds.push(new_bind);
                    }
                }
                // Take all new sub mounts and remove overlaps from all_binds while keeping the order
                let mut new_all_binds = vec!();
                for cur_bind in all_binds.into_iter() {
                    if ! bind_ref.dst.is_ancestor_of(&cur_bind.dst) {
                        new_all_binds.push(cur_bind);
                    }
                }
                new_all_binds.push_all(sub_binds.as_slice());
                all_binds = new_all_binds;
            }
        }
        Ok(all_binds)
    }

    fn add_tmpfs(&self, tmp: &TmpfsMount) -> io::IoResult<()> {
        let name = Path::new(match tmp.name {
            Some(n) => n,
            None => "tmpfs",
        });
        let flags = fs::MsFlags::empty();
        let dst = nested_dir!(self.root.dst, tmp.dst);
        let opt = "mode=0700";
        debug!("Creating tmpfs in {}", tmp.dst.display());
        try!(mount(&name, &dst, "tmpfs", &flags, &Some(opt)));
        Ok(())
    }

    // TODO: impl Drop to unmount and remove mount directories/files
    fn init_fs(&self) -> io::IoResult<()> {
        // Prepare to remove all parent mounts with a pivot
        // TODO: Add a path blacklist to hide some directories (e.g. when root.src == /)

        // TODO: Bind mount and seal the root before expanding bind mounts
        let all_binds = try!(self.expand_binds());
        for bind in all_binds.iter() {
            try!(self.add_bind(bind, false));
        }
        try!(change_dir(&self.root.dst));

        // TODO: Check all bind and tmpfs mount points consistency
        for tmp in self.tmps.iter() {
            try!(self.add_tmpfs(tmp));
        }

        // procfs
        let proc_src = Path::new("proc");
        let proc_dst = self.root.dst.clone().join(proc_src.clone());
        try!(mkdir_if_not(&proc_dst));
        let proc_flags = fs::MsFlags::empty();
        try!(mount(&proc_src, &proc_dst, "proc", &proc_flags, &None));

        // Devices
        try!(self.init_dev(&Path::new("/dev")));

        // Finalize the pivot
        let old_root = Path::new("tmp");
        try!(mkdir_if_not(&old_root));
        try!(pivot_root(&self.root.dst, &old_root));

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
                let fd = slave_fd.as_raw_fd();
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
            info!("Child jailing into {}", self.root.src.display());
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
                match unsafe { getuid() } {
                    0 => {}
                    _ => panic!("Fail to got root"),
                }
                // TODO: Expose the TTY
                match self.init_fs() {
                    Ok(_) => {}
                    Err(e) => panic!("Fail to initialize the file system: {}", e),
                }
                let groups = Vec::new();
                match setgroups(groups) {
                    Ok(_) => {}
                    Err(e) => panic!("Fail to set groups: {}", e),
                }

                // FIXME when using env* functions: task '<unnamed>' failed at 'could not initialize task_rng: couldn't open file (no such file or directory (No such file or directory); path=/dev/urandom; mode=open; access=read)', .../rust/src/libstd/rand/mod.rs:200
                //let env: Vec<(String, String)> = Vec::with_capacity(0);
                // XXX: Inherit HOME and TERM for now
                // TODO: Pass env from the client
                let env: Vec<(String, String)> = env().iter().filter_map(|&(ref n, ref v)| {
                    if n.as_slice() == "HOME" || n.as_slice() == "TERM" {
                        Some((n.clone(), v.clone()))
                    } else {
                        None
                    }
                }).collect();
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
