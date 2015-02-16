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

use std::old_io as io;
use std::old_io::FileType;
use std::old_io::fs::PathExtensions;

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
