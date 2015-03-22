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

extern crate libc;

use std::old_io as io;
use std::old_io::FileType;
use std::old_io::fs::PathExtensions;
use std::rand::{thread_rng, Rng};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use super::ns::{fs0, raw, umount};

/// Concatenate two paths (different from `join()`)
pub fn nest_path(root: &Path, subdir: &Path) -> Path {
    root.join(
        match subdir.path_relative_from(&Path::new("/")) {
            Some(p) => p,
            None => subdir.clone(),
        }
    )
}

#[test]
fn test_nest_path() {
    let foo = Path::new("/foo");
    let bar = Path::new("bar");
    let qux = Path::new("../qux");

    let foobar = Path::new("/foo/bar");
    assert_eq!(nest_path(&foo, &bar), foobar);
    let foofoo = Path::new("/foo/foo");
    assert_eq!(nest_path(&foo, &foo), foofoo);
    let barfoo = Path::new("bar/foo");
    assert_eq!(nest_path(&bar, &foo), barfoo);
    let barbar = Path::new("bar/bar");
    assert_eq!(nest_path(&bar, &bar), barbar);
    let fooqux = Path::new("/foo/../qux");
    assert_eq!(nest_path(&foo, &qux), fooqux);
    let barqux = Path::new("bar/../qux");
    assert_eq!(nest_path(&bar, &qux), barqux);
    let quxfoo = Path::new("../qux/foo");
    assert_eq!(nest_path(&qux, &foo), quxfoo);
    let quxbar = Path::new("../qux/bar");
    assert_eq!(nest_path(&qux, &bar), quxbar);
}

/// Do not return error if the directory already exist
pub fn mkdir_if_not(path: &Path) -> io::IoResult<()> {
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
pub fn touch_if_not(path: &Path) -> io::IoResult<()> {
    match path.stat() {
        Ok(fs) => match fs.kind {
            FileType::Directory =>
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

/// Create a `dst` file or directory according to the `src`
pub fn create_same_type(src: &Path, dst: &Path) -> io::IoResult<()> {
    match try!(src.stat()).kind {
        FileType::Directory => {
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

/// Create a directory and make it disappear when dropped.
/// This work even when the directory is in use.
pub struct EphemeralDir<'a> {
    delete_tx: Sender<()>,
    rel_path: Path,
    guard: Option<thread::JoinGuard<'a, ()>>,
}

#[cfg(target_os = "linux")]
impl<'a> EphemeralDir<'a> {
    pub fn new() -> EphemeralDir<'a> {
        let (tid_tx, tid_rx) = channel();
        let (delete_tx, delete_rx) = channel();
        let guard = thread::scoped(move || {
            // get[pt]id(2) are always successful
            let tid_path = format!("proc/{}/task/{}/fdinfo",
                                   unsafe { libc::getpid() },
                                   raw::gettid());
            let _ = tid_tx.send(tid_path);
            // Block to keep the ephemeral directory usable
            let _ = delete_rx.recv();
        });
        let rel_path = match tid_rx.recv() {
            Ok(v) => Path::new(v),
            Err(e) => panic!("Fail to create an ephemeral directory: {}", e),
        };
        EphemeralDir {
            delete_tx: delete_tx,
            rel_path: rel_path,
            guard: Some(guard),
        }
    }

    pub fn get_relative_path(&self) -> &Path {
        &self.rel_path
    }
}

#[unsafe_destructor]
impl<'a> Drop for EphemeralDir<'a> {
    fn drop(&mut self) {
        let _ = self.delete_tx.send(());
        if let Some(guard) = self.guard.take() {
            let _ = guard.join();
        }
    }
}

pub struct TmpWorkDir {
    path: Path,
    do_unmount: bool,
}

/// Create a temporary directory in the current directory and remove it when dropped
impl TmpWorkDir {
    // Can't use TempDir because it create an absolute path (through the removed workdir)
    pub fn new(prefix: &str) -> io::IoResult<Self> {
        let tmp_suffix: String = thread_rng().gen_ascii_chars().take(12).collect();
        let tmp_dir = Path::new(format!("./tmp_{}_{}", prefix, tmp_suffix));
        // With very bad luck, the command will failed :(
        try!(io::fs::mkdir(&tmp_dir, io::USER_RWX));
        Ok(TmpWorkDir {
            path: tmp_dir,
            do_unmount: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn unmount(&mut self, on: bool) {
        self.do_unmount = on;
    }
}

impl Drop for TmpWorkDir {
    fn drop(&mut self) {
        if self.do_unmount {
            match umount(&self.path, &fs0::MNT_DETACH) {
                Ok(..) => {}
                Err(e) => warn!("Fail to unmount {}: {}", self.path.display(), e),
            }
        }
        match io::fs::rmdir(&self.path) {
            Ok(..) => {}
            Err(e) => warn!("Fail to remove {}: {}", self.path.display(), e),
        }
        debug!("Removed {}", self.path.display());
    }
}
