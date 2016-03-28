// Copyright (C) 2015-2016 Mickaël Salaün
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

use bincode::rustc_serialize::{encode, decode};
use bincode::SizeLimit;
use rustc_serialize::{Encodable, Decodable};
use std::io::{Read, Write};

pub use stemflow::absolute_path;

// Message format: 2 bytes for the size + bincode encoding
pub fn send<T, U>(stream: &mut T, object: U) -> Result<(), String>
        where T: Write, U: Encodable {
    let encoded = match encode(&object, SizeLimit::Infinite) {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to encode request: {}", e)),
    };
    let mut encoded_size = [0u8; 2];
    if encoded.len() | 0xffff != 0xffff {
        return Err(format!("Failed to send request: Command too big"));
    }
    let size = encoded.len() as u16;
    for i in 0..2 {
        encoded_size[i] = (size >> (i * 8)) as u8;
    }
    match stream.write_all(&encoded_size) {
        Ok(_) => {}
        Err(e) => return Err(format!("Failed to send request: {}", e)),
    };
    match stream.write_all(encoded.as_ref()) {
        Ok(_) => Ok(()),
        Err(e) => return Err(format!("Failed to send request: {}", e)),
    }
}

pub fn recv<T, U>(stream: &mut T) -> Result<U, String>
        where T: Read, U: Decodable {
    let mut encoded_size = [0u8; 2];
    match stream.read_exact(&mut encoded_size) {
        Ok(_) => {}
        Err(e) => return Err(format!("Failed to read: {}", e)),
    }
    let mut size = 0u16;
    for i in 0..2 {
        size |= (encoded_size[i] as u16) << i;
    }
    // TODO: Add size limit (less than 64K)
    let mut encoded = Vec::with_capacity(size as usize);
    let encoded = match stream.take(size as u64).read_to_end(&mut encoded) {
        Ok(s) if s == size as usize => encoded,
        Ok(_) => return Err(format!("Failed to read all data")),
        Err(e) => return Err(format!("Failed to read: {}", e)),
    };
    match decode(encoded.as_ref()) {
        Ok(d) => Ok(d),
        Err(e) => Err(format!("Failed to decode: {:?}", e)),
    }
}
