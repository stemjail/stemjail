// Copyright (C) 2015 Mickaël Salaün
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

use rustc_serialize::{Encodable, Decodable, json};
use std::io::{BufRead, BufReader, Read, Write};

pub use stemflow::absolute_path;

pub fn send<T, U>(stream: &mut T, object: U) -> Result<(), String>
        where T: Write, U: Encodable {
    let encoded = match json::encode(&object) {
        Ok(s) => format!("{}\n", s),
        Err(e) => return Err(format!("Failed to encode request: {}", e)),
    };
    match stream.write_all(encoded.as_ref()) {
        Ok(_) => Ok(()),
        Err(e) => return Err(format!("Failed to send request: {}", e)),
    }
}

pub fn recv<T, U>(stream: &mut T) -> Result<U, String>
        where T: Read, U: Decodable {
    let mut encoded_str = String::new();
    let mut breader = BufReader::new(stream);
    match breader.read_line(&mut encoded_str) {
        Ok(_) => {}
        Err(e) => return Err(format!("Failed to read: {}", e)),
    }
    match json::decode(&encoded_str) {
        Ok(d) => Ok(d),
        Err(e) => Err(format!("Failed to decode JSON: {:?}", e)),
    }
}
