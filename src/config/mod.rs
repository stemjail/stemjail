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

extern crate rustc_serialize;
extern crate toml;

use self::rustc_serialize::Decodable;
use self::toml::Decoder;
use std::fs;
use std::path::Path;
use std::io::Read;

pub use self::error::ConfigError;

mod error;

pub mod portal;
pub mod profile;

// TODO: Check for absolute path only
pub fn get_config<T, U>(config_file: T) -> Result<U, ConfigError>
        where T: AsRef<Path>, U: Decodable {
    let mut contents = String::new();
    let _ = try!(fs::File::open(config_file)).read_to_string(&mut contents);
    let mut parser = toml::Parser::new(contents.as_ref());
    let toml = match parser.parse() {
        Some(r) => toml::Value::Table(r),
        None => return Err(ConfigError::new(format!("Parse error: {:?}", parser.errors))),
    };
    let mut decoder = Decoder::new(toml);
    let config = try!(Decodable::decode(&mut decoder));
    Ok(config)
}

pub fn get_configs<T, U>(profile_dir: T) -> Result<Vec<U>, ConfigError>
        where T: AsRef<Path>, U: Decodable {
    let mut ret = vec!();
    for file in try!(fs::read_dir(profile_dir)) {
        let file = try!(file).path();
        match file.extension() {
            Some(ext) => {
                if ext == "toml" {
                    match get_config(&file) {
                        Ok(c) => ret.push(c),
                        Err(e) => return Err(ConfigError::new(format!("(file `{}`) {}",
                                             file.display(), e))),
                    };
                }
            },
            None => {}
        }
    }
    Ok(ret)
}
