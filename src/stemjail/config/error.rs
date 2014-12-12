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
use std::io::IoError;
use super::toml::DecodeError;

pub struct ConfigError {
    pub detail: String,
}

impl ConfigError {
    pub fn new(detail: String) -> ConfigError {
        ConfigError {
            detail: detail,
        }
    }
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        "Configuration"
    }

    fn detail(&self) -> Option<String> {
        Some(self.detail.clone())
    }
}

impl FromError<DecodeError> for ConfigError {
    fn from_error(err: DecodeError) -> ConfigError {
        ConfigError::new(format!("Decode error: {}", err))
    }
}

impl FromError<IoError> for ConfigError {
    fn from_error(err: IoError) -> ConfigError {
        ConfigError::new(format!("I/O error: {}", err))
    }
}

impl fmt::Show for ConfigError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{} error: {}", self.description(), self.detail)
    }
}
