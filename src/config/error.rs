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

use std::error::Error;
use std::fmt;
use std::io;
use toml::DecodeError;

#[derive(Debug)]
pub struct ConfigError {
    desc: String,
}

impl ConfigError {
    pub fn new(detail: String) -> ConfigError {
        ConfigError {
            desc: format!("Configuration error: {}", detail),
        }
    }
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        self.desc.as_ref()
    }
}

impl From<DecodeError> for ConfigError {
    fn from(err: DecodeError) -> ConfigError {
        ConfigError::new(format!("Failed to decode: {}", err))
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> ConfigError {
        ConfigError::new(format!("I/O error: {}", err))
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}", self.desc)
    }
}
