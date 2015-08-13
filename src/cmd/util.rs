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

use bufstream::BufStream;
use rustc_serialize::{Encodable, Decodable, json};
use std::io::{BufRead, Write};
use std::path::Path;
use unix_socket::UnixStream;

// TODO: Replace with generic trait
macro_rules! impl_json {
    ($name: ty) => {
        impl $name {
            pub fn decode(encoded: &String) -> json::DecodeResult<$name> {
                json::decode(encoded.as_ref())
            }
            pub fn encode(&self) -> Result<String, json::EncoderError> {
                json::encode(&self)
            }
        }
    }
}

pub fn send<T>(bstream: &mut BufStream<UnixStream>, object: T) -> Result<(), String>
        where T: Encodable {
    let encoded = match json::encode(&object) {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to encode request: {}", e)),
    };
    match bstream.write_all(encoded.as_ref()) {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to send request: {}", e)),
    }
    match bstream.flush() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to flush request: {}", e)),
    }
}

pub fn recv<T>(bstream: &mut BufStream<UnixStream>) -> Result<T, String>
        where T: Decodable {
    let mut encoded_str = String::new();
    match bstream.read_line(&mut encoded_str) {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to read: {}", e)),
    };
    match json::decode(&encoded_str) {
        Ok(d) => Ok(d),
        Err(e) => Err(format!("Failed to decode JSON: {:?}", e)),
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
