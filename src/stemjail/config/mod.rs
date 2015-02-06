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

extern crate "rustc-serialize" as rustc_serialize;
extern crate toml;

use self::rustc_serialize::Decodable;
use self::toml::Decoder;
use std::old_io::{File, fs};

pub use self::error::ConfigError;

mod error;

pub mod portal;
pub mod profile;

// TODO: Check for absolute path only
pub fn get_config<T>(config_file: &Path) -> Result<T, ConfigError>
        where T: Decodable {
    let contents = try!(File::open(config_file).read_to_string());
    let mut parser = toml::Parser::new(contents.as_slice());
    let toml = match parser.parse() {
        Some(r) => toml::Value::Table(r),
        None => return Err(ConfigError::new(format!("Parse error: {:?}", parser.errors))),
    };
    let mut decoder = Decoder::new(toml);
    let config = try!(Decodable::decode(&mut decoder));
    Ok(config)
}

pub fn get_configs<T>(profile_dir: &Path) -> Result<Vec<T>, ConfigError>
        where T: Decodable {
    let mut ret = vec!();
    for file in try!(fs::walk_dir(profile_dir)) {
        match file.extension_str() {
            Some(ext) => {
                if ext == "toml" {
                    match get_config::<T>(&file) {
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
