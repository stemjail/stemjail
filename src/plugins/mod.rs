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

pub mod run;

pub enum KageAction {
    KageNop,
    PrintHelp,
    SendPortalCommand,
}

#[deriving(Decodable, Encodable, Show)]
pub enum PortalRequest {
    PortalNop,
    RequestFileDescriptor,
}

#[deriving(Decodable, Encodable)]
pub struct PortalAck {
    //pub result: Result<(), String>,
    pub request: PortalRequest,
}

#[deriving(Decodable, Encodable, Show)]
pub enum PortalPluginCommand {
    RunCommand(self::run::PortalRunCommand),
}

impl PortalPluginCommand {
    pub fn is_valid_request(&self, req: &PortalRequest) -> bool {
        match *self {
            RunCommand(ref c) => {
                match *req {
                    PortalNop => true,
                    RequestFileDescriptor => c.stdio,
                    //_ => false,
                }
            }
        }
    }
}

pub trait Plugin {
    fn get_name<'a>(&'a self) -> &'a String;
    fn get_usage(&self) -> String;
    fn get_portal_cmd(&self) -> Option<PortalPluginCommand>;
    fn init_client(&mut self, args: &Vec<String>) -> Result<KageAction, String>;
}

fn get_plugins() -> Vec<Box<Plugin>> {
    vec!(
        box self::run::RunPlugin::new() as Box<Plugin>,
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
