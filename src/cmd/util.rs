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

use std::path::Path;

pub use ::util::{recv, send};

// TODO: Replace with generic trait
macro_rules! impl_encdec {
    ($name: ty) => {
        impl $name {
            pub fn decode<T>(encoded: T) -> DecodingResult<$name> where T: AsRef<[u8]> {
                use bincode::rustc_serialize::decode;
                decode(encoded.as_ref())
            }
            pub fn encode(&self) -> EncodingResult<Vec<u8>> {
                use bincode::SizeLimit;
                use bincode::rustc_serialize::encode;
                // TODO: Set a limit
                encode(&self, SizeLimit::Infinite)
            }
        }
    }
}

macro_rules! get_path {
    ($matches: expr, $name: expr) => {
        match $matches.opt_str($name) {
            Some(s) => PathBuf::from(s),
            None => return Err(format!("Missing {} path", $name)),
        }
    }
}

/// Check for remaining useless arguments
macro_rules! check_remaining {
    ($matches: expr) => {
        if ! $matches.free.is_empty() {
            return Err("Unknown trailing argument".to_string());
        }
    }
}

/// Forbid use of "." (i.e. the parent domain root directory)
pub fn check_parent_path<T>(path: T) -> Result<(), String> where T: AsRef<Path> {
    let path = path.as_ref();
    if ! path.is_absolute() {
        return Err("The path is not absolute".to_string());
    }
    // TODO: Factore with jail.import_bind()
    if path.starts_with("/proc") {
        return Err("Access denied to /proc".to_string());
    }
    Ok(())
}
