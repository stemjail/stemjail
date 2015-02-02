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

use std::error::{Error, FromError};
use std::fmt;
use std::old_io::IoError;
use super::toml::DecodeError;

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
        self.desc.as_slice()
    }
}

impl FromError<DecodeError> for ConfigError {
    fn from_error(err: DecodeError) -> ConfigError {
        ConfigError::new(format!("Fail to decode: {}", err))
    }
}

impl FromError<IoError> for ConfigError {
    fn from_error(err: IoError) -> ConfigError {
        ConfigError::new(format!("I/O error: {}", err))
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}", self.desc)
    }
}
