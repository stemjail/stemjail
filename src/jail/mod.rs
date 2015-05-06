// Copyright (C) 2014-2015 Mickaël Salaün
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

#![allow(deprecated)]

use config::profile::JailDom;
use EVENT_TIMEOUT;
use ffi::ns::{fs, raw, sched};
use ffi::ns::{mount, pivot_root, unshare};
use libc;
use libc::{c_int, exit, fork, pid_t, getpid, setsid, getgid, getuid};
use mnt::{get_mount, get_submounts, MntOps, VecMountEntry};
use self::util::*;
use srv;
use std::borrow::Cow::{Borrowed, Owned};
use std::env;
use std::fmt::Debug;
use std::fs::{OpenOptions, create_dir, soft_link};
use std::io;
use std::io::{ErrorKind, Error, Write};
use std::old_io::{Command, pipe, process};
use std::old_io::{IoErrorKind, Reader, Writer};
use std::old_path::Path as OldPath;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{channel, Receiver, Select};
use std::thread;
use stemflow::{FileAccess, RcDomain};

pub use self::session::Stdio;

mod session;

pub mod util;

pub static WORKDIR_PARENT: &'static str = "./parent";

pub trait JailFn: Send + Debug {
    fn call(&mut self, &mut Jail);
}

// TODO: Add tmpfs prelude to not pollute the root

#[derive(Debug, Clone, PartialEq)]
pub struct BindMount {
    src: PathBuf,
    dst: PathBuf,
    writable: bool,
    from_parent: bool,
}

impl BindMount {
    pub fn new(source: PathBuf, destination: PathBuf) -> BindMount {
        BindMount {
            src: source,
            dst: destination,
            writable: false,
            from_parent: false,
        }
    }

    pub fn writable(mut self, writable: bool) -> BindMount {
        self.writable = writable;
        self
    }

    pub fn from_parent(mut self, from_parent: bool) -> BindMount {
        self.from_parent = from_parent;
        self
    }
}

#[derive(Clone)]
pub struct TmpfsMount<'a> {
    name: Option<&'a str>,
    dst: PathBuf,
    is_root: bool,
}

impl<'a> TmpfsMount<'a> {
    pub fn new(dst: PathBuf) -> TmpfsMount<'a> {
        TmpfsMount {
            name: None,
            dst: dst,
            is_root: false,
        }
    }

    pub fn name(mut self, name: &'a str) -> TmpfsMount<'a> {
        self.name = Some(name);
        self
    }

    pub fn is_root(mut self, is_root: bool) -> TmpfsMount<'a> {
        self.is_root = is_root;
        self
    }
}


// TODO: Add UUID
pub struct Jail<'a> {
    /// Jail description
    name: String,
    /// Destination root
    root: PathBuf,
    jdom: JailDom,
    tmps: Vec<TmpfsMount<'a>>,
    stdio: Option<Stdio>,
    pid: Arc<RwLock<Option<pid_t>>>,
    end_event: Option<Receiver<Result<(), ()>>>,
    workdir: Option<PathBuf>,
}

impl<'a> Jail<'a> {
    // TODO: Check configuration for duplicate binds entries and refuse to use it if so
    pub fn new(name: String, jdom: JailDom, tmps: Vec<TmpfsMount>) -> Jail {
        // TODO: Check for a real procfs
        Jail {
            name: name,
            root: PathBuf::from(format!("/proc/{}/fdinfo", unsafe { getpid() })),
            jdom: jdom,
            tmps: tmps,
            stdio: None,
            pid: Arc::new(RwLock::new(None)),
            end_event: None,
            workdir: None,
        }
    }

    // FIXME: Exclude /dev and /proc in the configurations
    pub fn gain_access(&mut self, acl: Vec<FileAccess>) -> Result<(), ()> {
        let acl = acl.into_iter().map(|x| Arc::new(x)).collect();
        match self.jdom.dom.reachable(&acl) {
            Some(dom) => {
                // TODO: Compare the reference
                if dom == self.jdom.dom {
                    debug!("Current domain already allow this access");
                    return Ok(());
                }
                let prev = self.jdom.clone();
                self.jdom = dom.into();
                // TODO: Optimize with intersection
                let binds = self.jdom.binds.iter().filter(|&x|
                    prev.binds.iter().find(|&y| *y == *x).is_none()
                ).map(|x| {
                    let mut b = x.clone();
                    b.from_parent = true;
                    b
                });
                for bind in binds {
                    // FIXME: Check transition result and restore to previous state if any error
                    // FIXME: Do all mounts in the workdir and if all OK, move them in the jail
                    let _ = self._import_bind(&bind, true);
                }
                debug!("Domain transition: {} -> {}", prev.dom.name, self.jdom.dom.name);
                Ok(())
            }
            None => {
                debug!("No domain reachable");
                Err(())
            }
        }
    }

    /// Map the current user to himself
    fn init_userns(&self, pid: pid_t) -> io::Result<()> {
        // Do not use write/format_args_method-like macros, proc files must be
        // write only at once to avoid invalid argument.
        let mut file = OpenOptions::new();
        file.write(true);
        let mut uid_file = try!(file.open(format!("/proc/{}/uid_map", pid)));
        try!(uid_file.write_all(format!("{0} {0} 1", unsafe { getuid() }).as_bytes()));
        match file.open(format!("/proc/{}/setgroups", pid)) {
            Ok(mut setgroups_file) => try!(setgroups_file.write_all("deny".as_bytes())),
            Err(e) => if e.kind() != ErrorKind::NotFound {
                return Err(e);
            }
        }
        let mut gid_file = try!(file.open(format!("/proc/{}/gid_map", pid)));
        // TODO: Keep the current group mapping
        try!(gid_file.write_all(format!("{0} {0} 1", unsafe { getgid() }).as_bytes()));
        Ok(())
    }

    fn init_dev<T>(&self, devdir: T) -> io::Result<()> where T: AsRef<Path> {
        let devdir = devdir.as_ref();
        info!("Populating {}", devdir.display());
        let devdir_full = nest_path(&self.root, &devdir);
        try!(mkdir_if_not(&devdir_full));
        try!(self.add_tmpfs(&TmpfsMount::new(devdir.to_path_buf()).name("dev")));

        // TODO: Use macro
        // Create mount points
        let devs = &[
            "null",
            "zero",
            "full",
            "urandom",
            ];
        let mut devs: Vec<BindMount> = devs.iter().map(|dev| {
            let src = devdir.join(dev);
            BindMount::new(src.clone(), src).writable(true)
        }).collect();

        // Add current TTY
        // TODO: Add dynamic TTY list reload
        match self.stdio {
            // FIXME: Assume `s` begin with "/dev/"
            Some(ref s) => {
                let p = s.as_ref();
                devs.push(BindMount::new(p.to_path_buf(), p.to_path_buf()).writable(true));
            }
            None => {}
        }

        for dev in devs.iter() {
            debug!("Creating {}", dev.dst.display());
            let bind = BindMount::new(dev.src.clone(), nest_path(&self.root, &dev.dst))
                .writable(true);
            try!(self.add_bind(&bind, true));
        }
        let links = &[
            ("fd", "/proc/self/fd"),
            ("random", "urandom")
            ];
        for &(dst, src) in links.iter() {
            let dst = devdir_full.join(dst);
            try!(soft_link(src, dst));
        }
        try!(self.add_tmpfs(&TmpfsMount::new(devdir.join("shm")).name("shm")));

        // Seal /dev
        // TODO: Drop the root user to realy seal something…
        let dev_flags = fs::MS_BIND | fs::MS_REMOUNT | fs::MS_RDONLY;
        try!(mount("none", devdir_full, "", &dev_flags, &None));

        Ok(())
    }

    pub fn import_bind(&self, bind: &BindMount) -> io::Result<()> {
        // Do not create destination mount point
        self._import_bind(bind, false)
    }

    // FIXME: Handle non-directory mount
    fn _import_bind(&self, bind: &BindMount, create_dst: bool) -> io::Result<()> {
        let workdir = match self.workdir {
            Some(ref w) => w,
            // TODO: Create a new error or a FSM for self.workdir
            None => return Err(io::Error::new(ErrorKind::Other, "No workdir")),
        };
        // FIXME: Verify path traversal sanitization (e.g. no "..")
        let parent = workdir.join(WORKDIR_PARENT);
        let (excludes, tmp_bind) = if bind.from_parent {
            // Protect parent process and dev listing
            // FIXME: Force bind.src to be an absolute path
            // TODO: Factore with cmd/shim
            match self.protected_paths().iter().find(|x| bind.src.starts_with(x)) {
                Some(d) => {
                    warn!("Access denied to parent {}", d.display());
                    return Err(io::Error::new(ErrorKind::PermissionDenied, "Access denied"));
                }
                None => {}
            }
            // Relative path for src
            let mut tmp_bind = bind.clone();
            // Virtual source path to check sub mounts
            tmp_bind.src = nest_path(&parent, &bind.src);
            (vec!(), tmp_bind)
        } else {
            (vec!(workdir.clone()), bind.clone())
        };
        // Deny some directories from being masked
        // FIXME: Force bind.dst to be an absolute path
        match self.protected_paths().iter().find(|x| bind.dst.starts_with(x)) {
            Some(d) => {
                warn!("Can't overlaps {}", d.display());
                return Err(io::Error::new(ErrorKind::PermissionDenied, "Can't overlays"));
            }
            None => {}
        }
        // Create temporary and unique directory for an atomic cmd/mount command
        let mut tmp_dir = try!(TmpWorkDir::new("mount"));

        let submounts = try!(self.expand_binds(vec!(tmp_bind), &excludes.iter().collect()));
        for mount in submounts.iter() {
            let mut mount = mount.clone();
            if mount.from_parent {
                let rel_src = mount.src.clone();
                let rel_src = match rel_src.relative_from(&parent) {
                    Some(p) => p,
                    None => {
                        warn!("Failed to get relative path from {}", parent.display());
                        return Err(io::Error::new(ErrorKind::Other, "Relative path conversion"));
                    }
                };
                mount.src = nest_path(&WORKDIR_PARENT, rel_src);
            }
            let rel_dst = mount.dst.clone();
            let rel_dst = match rel_dst.relative_from(&bind.dst) {
                Some(p) => p,
                None => {
                    warn!("Failed to get relative path from {}", bind.dst.display());
                    return Err(io::Error::new(ErrorKind::Other, "Relative path conversion"));
                }
            };
            mount.dst = nest_path(&tmp_dir, rel_dst);
            match self.add_bind(&mount, true) {
                Ok(..) => {
                    // Unmount all previous mounts if an error occured
                    tmp_dir.unmount(true);
                }
                Err(e) => {
                    warn!("Failed to bind mount a submount point: {}", e);
                    return Err(e);
                }
            }
        }

        debug!("Moving bind mount from {} to {}", tmp_dir.as_ref().display(), bind.dst.display());
        if create_dst {
            try!(create_same_type(&tmp_dir, &bind.dst));
        }
        match mount(&tmp_dir, &bind.dst, "none", &fs::MS_MOVE, &None) {
            Ok(..) => tmp_dir.unmount(false),
            Err(e) => {
                warn!("Failed to move the temporary mount point: {}", e);
                return Err(e);
            }
        }
        Ok(())
    }

    // XXX: Impossible to keep a consistent read-only mount tree if a new mount is added after our
    // bind mount. Will need to watch all the sources.
    // TODO: Try to not bind remount already read-only mounts
    fn add_bind(&self, bind: &BindMount, is_absolute: bool) -> io::Result<()> {
        let dst = if is_absolute {
            Borrowed(&bind.dst)
        } else {
            Owned(nest_path(&self.root, &bind.dst))
        };
        let dst = &*dst;
        let src = &bind.src;

        // TODO: Add better log (cf. parent)
        debug!("Bind mounting from {}", src.display());
        debug!("Bind mounting to {}", dst.display());

        // Create needed directorie(s) and/or file
        // XXX: This should be allowed for clients too
        // FIXME: dst is a temporary destination!
        try!(create_same_type(src, dst));

        let none_str = "none";
        // The fs/namespace.c:clone_mnt kernel function forbid unprivileged users (i.e.
        // CL_UNPRIVILEGED) to reveal what is under a mount, so we need to recursively bind mount.
        let bind_flags = fs::MS_BIND | fs::MS_REC;
        try!(mount(src, dst, none_str, &bind_flags, &None));

        if ! bind.writable {
            // When write action is forbiden we must not use the MS_REC to avoid unattended
            // read/write files during the jail life.
            let none_path = "none";
            // Seal the vfsmount: good to not receive new mounts but block unmount as well
            // TODO: Add a "unshare <path>" command to remove a to-be-unmounted path
            let bind_flags = fs::MS_PRIVATE | fs::MS_REC;
            try!(mount(&none_path, dst, none_str, &bind_flags, &None));

            // Take the same mount flags as the source
            let flags = match get_mount(&src) {
                Ok(Some(mount)) => mount.mntops.iter().filter_map(|x| {
                    // Cf. linux/fs/namespace.c:do_remount
                    match *x {
                        MntOps::Atime(false) => Some(fs::MS_NOATIME),
                        MntOps::DirAtime(false) => Some(fs::MS_NODIRATIME),
                        MntOps::RelAtime(true) => Some(fs::MS_RELATIME),
                        MntOps::Dev(false) => Some(fs::MS_NODEV),
                        MntOps::Exec(false) => Some(fs::MS_NOEXEC),
                        MntOps::Suid(false) => Some(fs::MS_NOSUID),
                        MntOps::Write(false) => Some(fs::MS_RDONLY),
                        _ => None,
                    }
                }).fold(fs::MsFlags::empty(), |x, y| x | y),
                _ => fs::MsFlags::empty(),
            };
            // Remount read-only, even if the source is already read-only, to be sure to control
            // the destination mount point properties during all its life (e.g. the parent
            // namespace can remount the source read-write).
            let bind_flags = fs::MS_BIND | fs::MS_REMOUNT | fs::MS_RDONLY | flags;
            try!(mount(&none_path, dst, none_str, &bind_flags, &None));
        }
        Ok(())
    }

    fn expand_binds<T>(&self, binds: Vec<BindMount>, excludes: &Vec<T>)
            -> io::Result<Vec<BindMount>> where T: AsRef<Path> {
        let host_mounts: Vec<_> = match get_submounts("/") {
            Ok(list) => {
                let proc_path = "/proc";
                // Exclude workdir from overlaps detection because workdir/parent contains moved
                // mount points and is so at the top of the mount list.
                let excludes_overlaps: Vec<&AsRef<Path>> = match self.workdir {
                    None => vec!(),
                    Some(ref w) => vec!(&proc_path, w),
                };
                // FIXME: Verify remove_overlaps() implementation for missed mount points
                list.remove_overlaps(&excludes_overlaps).into_iter().filter(
                    |mount| {
                        excludes.iter().skip_while(
                            |path| !mount.file.starts_with(path)
                        ).next().is_none()
                    }).collect()
            },
            Err(e) => {
                // TODO: Add FromError impl to io::Result
                warn!("Failed to get mount points: {}", e);
                return Err(io::Error::new(ErrorKind::NotFound, "No mount point found"))
            }
        };

        // Need to keep the mount points order and prioritize the last (i.e. user) mount points
        let mut all_binds: Vec<BindMount> = vec!();
        for bind in binds.into_iter() {
            // FIXME: Extend the full path (like "readlink -f") to not recursively mount
            let sub_binds = if bind.writable {
                vec!(bind.clone())
            } else {
                // Complete with all child mount points if needed (i.e. read-only mount tree)
                let mut sub_binds = vec!(bind.clone());
                // Take bind sub mounts
                for mount in host_mounts.iter() {
                    let sub_src = mount.file.clone();
                    if sub_src.starts_with(&bind.src) && bind.src != sub_src {
                        let rel_dst = match mount.file.relative_from(&bind.src) {
                            Some(p) => p,
                            None => {
                                warn!("Failed to get relative path from {}", bind.src.display());
                                return Err(io::Error::new(ErrorKind::Other, "Relative path conversion"));
                            }
                        };
                        // Extend bind with same attributes
                        let new_bind = BindMount::new(sub_src, nest_path(&bind.dst, &rel_dst))
                            .writable(bind.writable).from_parent(bind.from_parent);
                        sub_binds.push(new_bind);
                    }
                }
                sub_binds
            };
            // While keeping the previous mounts order, drop those who would be overlapped by the
            // current bind
            let mut new_all_binds = vec!();
            for cur_bind in all_binds.into_iter() {
                if ! cur_bind.dst.starts_with(&bind.dst) {
                    new_all_binds.push(cur_bind);
                }
            }
            new_all_binds.push_all(sub_binds.as_slice());
            all_binds = new_all_binds;
        }
        Ok(all_binds)
    }

    fn add_tmpfs(&self, tmp: &TmpfsMount) -> io::Result<()> {
        let name = PathBuf::from(match tmp.name {
            Some(n) => n,
            None => "tmpfs",
        });
        let flags = fs::MsFlags::empty();
        debug!("Creating tmpfs in {}", tmp.dst.display());
        let dst = if tmp.is_root {
            tmp.dst.clone()
        } else {
            nest_path(&self.root, &tmp.dst)
        };
        let opt = "mode=0700";
        try!(mkdir_if_not(&dst));
        try!(mount(&name, &dst, "tmpfs", &flags, &Some(opt)));
        Ok(())
    }

    fn protected_paths(&self) -> Vec<&Path> {
        // Protect custom procfs and devices
        vec!(Path::new("/dev"), Path::new("/proc"))
    }

    // TODO: impl Drop to unmount and remove mount directories/files
    fn init_fs(&mut self) -> io::Result<()> {
        // Create an empty and writable root to be able to add any bind mounts
        // FIXME: Seal the root
        try!(self.add_tmpfs(&TmpfsMount::new(self.root.clone()).name("root").is_root(true)));

        // Prepare to remove all parent mounts with a pivot
        let all_binds = try!(self.expand_binds(self.jdom.binds.clone(), &{
            let mut exclude = self.protected_paths();
            exclude.push(self.root.as_ref());
            exclude
        }));
        for bind in all_binds.iter() {
            try!(self.add_bind(bind, false));
        }
        try!(env::set_current_dir(&self.root));

        // TODO: Check all bind and tmpfs mount points consistency
        for tmp in self.tmps.iter() {
            try!(self.add_tmpfs(tmp));
        }

        // procfs
        let proc_src = "proc";
        let proc_dst = self.root.join(&proc_src);
        try!(mkdir_if_not(&proc_dst));
        let proc_flags = fs::MsFlags::empty();
        try!(mount(&proc_src, &proc_dst, "proc", &proc_flags, &None));

        // Devices
        try!(self.init_dev("/dev"));

        // Prepare a private working directory
        let pid = unsafe { getpid() };
        let workdir = PathBuf::from(format!("proc/{}/fdinfo", pid));

        // Backup the original proc entry
        let workdir_bkp = PathBuf::from(format!("proc/{}/fd", pid));
        let bind = BindMount::new(workdir.clone(), workdir_bkp.clone()).writable(true);
        try!(self.add_bind(&bind, true));

        // Save the workdir path to be able to exclude it from mount points
        let workdir_abs = Path::new("/").join(&workdir);
        self.workdir = Some(workdir_abs.clone());

        // Create the monitor working directory
        try!(self.add_tmpfs(&TmpfsMount::new(PathBuf::from(&workdir)).name("monitor")));
        let parent = workdir.join(WORKDIR_PARENT);
        // FIXME: Set umask to !io::USER_RWX
        try!(create_dir(&parent));

        // TODO: Bind mount the parent root to be able to drop mount branches (i.e. domain transitions)
        try!(pivot_root(&self.root, &parent));

        // Keep the workdir open (e.g. jail transitions)
        try!(env::set_current_dir(&workdir));

        // Hide the workdir
        try!(mount(&Path::new("/").join(&workdir_bkp), &workdir_abs, "none", &fs::MS_MOVE, &None));
        Ok(())
    }

    // TODO: Return io::Result<()>
    pub fn run<T>(&mut self, run: T, args: &Vec<String>, stdio: Option<Stdio>) where T: AsRef<Path> {
        info!("Running jail {}", self.name);

        // TODO: Replace fork with a new process creation and dedicated protocol
        // Fork a new process
        let mut sync_parent = match pipe::PipeStream::pair() {
            Ok(p) => p,
            Err(e) => panic!("Failed to create pipe #1: {}", e),
        };
        let mut sync_child = match pipe::PipeStream::pair() {
            Ok(p) => p,
            Err(e) => panic!("Failed to create pipe #2: {}", e),
        };
        let (mut jail_pid_rx, mut jail_pid_tx) = match pipe::PipeStream::pair() {
            Ok(p) => (p.reader, p.writer),
            Err(e) => panic!("Failed to create pipe #3: {}", e),
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
                    process::InheritFd(fd),
                    process::InheritFd(fd),
                    process::InheritFd(fd),
                )
            },
            None => {(
                None,
                process::InheritFd(libc::STDIN_FILENO),
                process::InheritFd(libc::STDOUT_FILENO),
                process::InheritFd(libc::STDERR_FILENO),
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
            panic!("Failed to fork #1");
        } else if pid == 0 {
            // Child
            drop(jail_pid_rx);
            info!("Child jailing");
            // Become a process group leader
            // TODO: Change behavior for dedicated TTY
            match unsafe { setsid() } {
                -1 => panic!("Failed to create a new session: {}", Error::last_os_error()),
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
                Err(e) => panic!("Failed to unshare: {}", e),
            }

            // Sync with parent
            match sync_parent.writer.write_i8(0) {
                Ok(_) => {}
                Err(e) => panic!("Failed to synchronise with parent #1: {}", e),
            }
            match sync_child.reader.read_i8() {
                Ok(_) => {}
                Err(e) => panic!("Failed to synchronise with parent #2: {}", e),
            }

            // Need to fork because of the PID namespace and the group ID
            let pid = unsafe { fork() };
            if pid < 0 {
                panic!("Failed to fork #2");
            } else if pid == 0 {
                // Child
                // TODO: Expose the TTY
                match self.init_fs() {
                    Ok(_) => {}
                    Err(e) => panic!("Failed to initialize the file system: {}", e),
                }
                // A normal user must not be able to drop groups to avoid permission bypass (cf.
                // user_namespaces(7): the setgroups file)

                // FIXME when using env* functions: task '<unnamed>' failed at 'could not initialize task_rng: couldn't open file (no such file or directory (No such file or directory); path=/dev/urandom; mode=open; access=read)', .../rust/src/libstd/rand/mod.rs:200
                // XXX: Inherit HOME and TERM for now
                let env: Vec<(String, String)> = env::vars().filter_map(|(ref n, ref v)| {
                    match n.as_slice() {
                        "HOME" | "TERM" => Some((n.clone(), v.clone())),
                        _ => None,
                    }
                }).collect();
                // TODO: Try using detached()
                let mut process = match Command::new(run.as_ref().to_string_lossy().as_slice())
                        // Must switch to / to avoid leaking hidden parent root
                        .cwd(&OldPath::new("/"))
                        .stdin(stdin)
                        .stdout(stdout)
                        .stderr(stderr)
                        .env_set_all(env.as_slice())
                        .args(args.as_slice())
                        .spawn() {
                    Ok(p) => p,
                    Err(e) => panic!("Failed to execute process: {}", e),
                };
                // Need to keep the slave TTY open until passing to the child
                drop(slave_fd.take());
                // TODO: Check 32-bits compatibility with other arch
                match jail_pid_tx.write_le_i32(process.id()) {
                    Ok(_) => {}
                    Err(e) => panic!("Failed to send child PID: {}", e),
                }
                drop(jail_pid_tx);
                // TODO: Forward the ProcessExit to the jail object

                let quit = Arc::new(AtomicBool::new(false));
                let (cmd_tx, cmd_rx) = channel();
                let (child_tx, child_rx) = channel();

                let events = Select::new();
                // Handle events from cmd::* using self
                let mut cmd_handle = events.handle(&cmd_rx);
                unsafe { cmd_handle.add() };
                // Handle child wait event
                let mut child_handle = events.handle(&child_rx);
                unsafe { child_handle.add() };

                let cmd_quit = quit.clone();
                let cmd_thread = thread::scoped(move || {
                    srv::monitor_listen(cmd_tx, cmd_quit);
                });

                let child_thread = thread::scoped(move || {
                    'main: loop {
                        process.set_timeout(EVENT_TIMEOUT);
                        let child_ret = process.wait();
                        match child_ret {
                            Ok(ret) => {
                                debug!("Jail child (PID {}) exited with {}", process.id(), ret);
                                break 'main;
                            }
                            Err(ref e) if e.kind == IoErrorKind::TimedOut => {}
                            Err(e) => {
                                warn!("Failed to wait for child (PID {}): {}", process.id(), e);
                                let _ = process.signal_kill();
                                break 'main;
                            }
                        }
                    }
                    let _ = child_tx.send(());
                });

                // Wait for client commands and child event
                'main: loop {
                    let event = events.wait();
                    if event == cmd_handle.id() {
                        match cmd_handle.recv() {
                            Ok(mut f) => f.call(self),
                            Err(e) => warn!("Failed to receive the command: {}", e),
                        }
                    } else if event == child_handle.id() {
                        match child_handle.recv() {
                            Ok(..) => break 'main,
                            Err(e) => warn!("Failed to receive the command: {}", e),
                        }
                    } else {
                        panic!("Received unknown event");
                    }
                }

                quit.store(true, Relaxed);
                let _ = child_thread.join();
                debug!("Jail child monitor exited");
                let _ = cmd_thread.join();
                debug!("Jail command monitor exited");
                unsafe { exit(0); }
            } else {
                // Parent
                drop(jail_pid_tx);
                drop(slave_fd.take());
                let mut status: c_int = 0;
                // TODO: Replace waitpid(2) with wait(2)
                let _ = unsafe { raw::waitpid(pid, &mut status, 0) };
                unsafe { exit(0); }
            }
        } else {
            // Parent
            drop(jail_pid_tx);
            drop(slave_fd.take());
            // TODO: Send fail command to the child if any error
            let _ = sync_parent.reader.read_i8();
            match self.init_userns(pid) {
                Ok(_) => {}
                Err(e) => panic!("Failed to initialize user namespace: {}", e),
            }
            match sync_child.writer.write_i8(0) {
                Ok(_) => {}
                Err(e) => panic!("Failed to synchronise with child: {}", e),
            }
            // Get the child PID
            match jail_pid_rx.read_le_i32() {
                Ok(p) => {
                    let mut lock = match jail_pid.write() {
                        Ok(g) => g,
                        Err(e) => panic!("Failed to save the jail PID: {:?}", e),
                    };
                    *lock = Some(p);
                }
                Err(e) => panic!("Failed to get jail PID: {}", e),
            }
            debug!("Got jail PID: {}", {
                match jail_pid.read() {
                    Ok(v) => v.unwrap_or(-1),
                    Err(e)=> panic!("Failed to read the jail PID: {:?}", e),
            }});
            debug!("Waiting for child {} to terminate", pid);
            thread::spawn(move || {
                let mut status: c_int = 0;
                // TODO: Replace waitpid(2) with wait(2)
                match unsafe { raw::waitpid(pid, &mut status, 0) } {
                    //-1 => panic!("Failed to wait for child {}", pid),
                    -1 => drop(end_tx.send(Err(()))),
                    _ => { {
                            let mut lock = match jail_pid.write() {
                                Ok(g) => g,
                                Err(e) => panic!("Failed to reset the jail PID: {:?}", e),
                            };
                            *lock = None;
                        }
                        drop(end_tx.send(Ok(())));
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
            &Some(ref event) =>  match event.recv() {
                Ok(_) => Ok(()),
                Err(_) => Err(()),
            },
            &None => Err(()),
        }
    }
}
