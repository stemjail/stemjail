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

pub enum PluginCommand {
    Nop,
    PrintHelp,
}

pub trait Plugin {
    fn get_name<'a>(&'a self) -> &'a String;
    fn get_usage(&self) -> String;
    fn init_client(&self, args: &Vec<String>) -> Result<PluginCommand, String>;
}

fn get_plugins() -> Vec<Box<Plugin>> {
    vec!(
        self::run::get_plugin(),
    )
}

pub fn get_plugin(name: &String) -> Option<Box<Plugin>> {
    for plugin in get_plugins().move_iter() {
        if plugin.get_name() == name {
            return Some(plugin);
        }
    }
    None
}

pub fn get_plugins_name() -> Vec<String> {
    get_plugins().iter().map(|x| x.get_name().clone()).collect()
}
