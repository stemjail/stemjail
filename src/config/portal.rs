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

use std::fmt;
use super::profile::ProfileConfig;

pub struct Portal {
    configs: Vec<ProfileConfig>,
}

impl Portal {
    pub fn new(configs: Vec<ProfileConfig>) -> Portal {
        Portal {
            configs: configs,
        }
    }

    pub fn profile<T>(&self, name: T) -> Option<&ProfileConfig> where T: AsRef<str> {
        self.configs.iter().find(|c| { AsRef::<str>::as_ref(&c.name) == name.as_ref() })
    }
}

impl fmt::Display for Portal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "profiles: {:?}", self.configs.iter().map(|x| &x.name ).collect::<Vec<_>>())
    }
}
