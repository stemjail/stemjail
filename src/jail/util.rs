// Copyright (C) 2014-2016 Mickaël Salaün
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

use ffi::ns::{fs0, umount};
use rand::{Rng, thread_rng};
use std::fs::{File, create_dir, create_dir_all, remove_dir};
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Concatenate two paths (different from `join()`)
pub fn nest_path<T, U>(root: T, subdir: U) -> PathBuf where T: AsRef<Path>, U: AsRef<Path> {
    let root = root.as_ref();
    let subdir = subdir.as_ref();
    root.join(
        match subdir.relative_from("/") {
            Some(p) => p.to_path_buf(),
            None => subdir.to_path_buf(),
        }
    )
}

#[test]
fn test_nest_path() {
    let foo = "/foo";
    let bar = "bar";
    let qux = "../qux";

    let foobar = PathBuf::from("/foo/bar");
    assert_eq!(nest_path(&foo, &bar), foobar);
    let foofoo = PathBuf::from("/foo/foo");
    assert_eq!(nest_path(&foo, &foo), foofoo);
    let barfoo = PathBuf::from("bar/foo");
    assert_eq!(nest_path(&bar, &foo), barfoo);
    let barbar = PathBuf::from("bar/bar");
    assert_eq!(nest_path(&bar, &bar), barbar);
    let fooqux = PathBuf::from("/foo/../qux");
    assert_eq!(nest_path(&foo, &qux), fooqux);
    let barqux = PathBuf::from("bar/../qux");
    assert_eq!(nest_path(&bar, &qux), barqux);
    let quxfoo = PathBuf::from("../qux/foo");
    assert_eq!(nest_path(&qux, &foo), quxfoo);
    let quxbar = PathBuf::from("../qux/bar");
    assert_eq!(nest_path(&qux, &bar), quxbar);
}

/// Do not return error if the directory already exist
pub fn mkdir_if_not<T>(path: T) -> io::Result<()> where T: AsRef<Path> {
    // FIXME: Set umask to !io::USER_RWX
    match create_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) => match e.kind() {
            io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(e)
        },
    }
}

/// Do not return error if the file already exist
pub fn touch_if_not<T>(path: T) -> io::Result<()> where T: AsRef<Path> {
    match path.as_ref().metadata() {
        Ok(md) => {
            if md.is_dir() {
                Err(io::Error::new(ErrorKind::InvalidInput, "Directory"))
            } else {
                Ok(())
            }
        }
        Err(e) => match e.kind() {
            ErrorKind::NotFound => match File::create(path) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            },
            _ => Err(e),
        },
    }
}

/// Create a `dst` file or directory according to the `src`
pub fn create_same_type<T, U>(src: T, dst: U) -> io::Result<()>
        where T: AsRef<Path>, U: AsRef<Path> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if try!(src.metadata()).is_dir() {
        try!(mkdir_if_not(dst));
    } else {
        let mut d = dst.to_path_buf();
        d.pop();
        try!(mkdir_if_not(&d));
        try!(touch_if_not(dst));
    }
    Ok(())
}

// TODO: Handle temporary file (e.g. bind mount a file)
pub struct TmpWorkDir {
    path: PathBuf,
    do_unmount: bool,
}

/// Create a temporary directory in the current directory and remove it when dropped
impl TmpWorkDir {
    // Can't use TempDir because it create an absolute path (through the removed workdir)
    pub fn new(prefix: &str) -> io::Result<Self> {
        let tmp_suffix: String = thread_rng().gen_ascii_chars().take(12).collect();
        let tmp_dir = PathBuf::from(format!("./tmp_{}_{}", prefix, tmp_suffix));
        // With very bad luck, the command will fail :(
        // FIXME: Set umask to !io::USER_RWX
        try!(create_dir(&tmp_dir));
        Ok(TmpWorkDir {
            path: tmp_dir,
            do_unmount: false,
        })
    }

    pub fn unmount(&mut self, on: bool) {
        self.do_unmount = on;
    }
}

impl AsRef<Path> for TmpWorkDir {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Drop for TmpWorkDir {
    fn drop(&mut self) {
        if self.do_unmount {
            match umount(&self.path, &fs0::MNT_DETACH) {
                Ok(..) => {}
                Err(e) => warn!("Failed to unmount {}: {}", self.path.display(), e),
            }
        }
        match remove_dir(&self.path) {
            Ok(..) => debug!("Removed {}", self.path.display()),
            Err(e) => warn!("Failed to remove {}: {}", self.path.display(), e),
        }
    }
}
