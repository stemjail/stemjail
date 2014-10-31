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

extern crate serialize;
extern crate toml;

use self::toml::{Decoder, DecodeError};
use serialize::Decodable;
use std::io::File;

pub mod profile;

// TODO: Check for absolute path only
pub fn get_config<T>(config_file: &Path) -> Result<T, String>
        where T: Decodable<Decoder, DecodeError> {
    let contents = match File::open(config_file).read_to_string() {
        Ok(r) => r,
        Err(e) => return Err(format!("Error reading config file: {}", e)),
    };
    let mut parser = toml::Parser::new(contents.as_slice());
    let toml = match parser.parse() {
        Some(r) => toml::Table(r),
        None => return Err(format!("Error parsing config file: {}", parser.errors)),
    };
    let mut decoder = Decoder::new(toml);
    let config = match Decodable::decode(&mut decoder) {
        Ok(r) => r,
        Err(e) => return Err(format!("Error decoding config file: {}", e)),
    };
    Ok(config)
}
