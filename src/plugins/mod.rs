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

#[derive(Copy)]
pub enum KageAction {
    Nop,
    PrintHelp,
    SendPortalCommand,
}

#[derive(Copy, RustcDecodable, RustcEncodable, Show)]
pub enum PortalRequest {
    Nop,
    CreateTty,
}

#[derive(Copy, RustcDecodable, RustcEncodable, Show)]
pub struct PortalAck {
    //pub result: Result<(), String>,
    pub request: PortalRequest,
}

#[derive(RustcDecodable, RustcEncodable, Show)]
pub enum PluginCommand {
    Run(self::run::RunCommand),
}

impl PluginCommand {
    pub fn is_valid_request(&self, req: &PortalRequest) -> bool {
        match *self {
            PluginCommand::Run(ref c) => {
                match *req {
                    PortalRequest::Nop => true,
                    PortalRequest::CreateTty => c.stdio,
                    //_ => false,
                }
            }
        }
    }
}

pub trait Plugin {
    fn get_name<'a>(&'a self) -> &'a String;
    fn get_usage(&self) -> String;
    fn get_portal_cmd(&self) -> Option<PluginCommand>;
    fn init_client(&mut self, args: &Vec<String>) -> Result<KageAction, String>;
}

fn get_plugins<'a>() -> Vec<Box<Plugin + 'a>> {
    vec!(
        box self::run::RunPlugin::new() as Box<Plugin>,
    )
}

pub fn get_plugin(name: &String) -> Option<Box<Plugin>> {
    for plugin in get_plugins().into_iter() {
        if plugin.get_name() == name {
            return Some(plugin);
        }
    }
    None
}

pub fn get_plugins_name() -> Vec<String> {
    get_plugins().iter().map(|x| x.get_name().clone()).collect()
}
