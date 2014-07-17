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

mod run;

struct Plugin<'a> {
    pub name: &'a str,
    pub init: fn(&Vec<String>) -> Result<(), ()>,
}

pub fn get_plugins() -> Vec<Plugin> {
    vec!(
        Plugin { name: "run", init: self::run::init },
    )
}

pub fn command(cmd: &String, args: &Vec<String>) -> Result<(), ()> {
    for plugin in get_plugins().iter() {
        if plugin.name == cmd.as_slice() {
            let init = plugin.init;
            return init(args);
        }
    }
    Err(())
}
