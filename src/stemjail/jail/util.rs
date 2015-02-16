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
use std::sync::mpsc::{channel, Sender};
use std::thread::{JoinGuard, Thread};
use super::ns::raw;

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
    guard: Option<JoinGuard<'a, ()>>,
}

#[cfg(target_os = "linux")]
impl<'a> EphemeralDir<'a> {
    pub fn new() -> EphemeralDir<'a> {
        let (tid_tx, tid_rx) = channel();
        let (delete_tx, delete_rx) = channel();
        let guard = Thread::scoped(move || {
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
