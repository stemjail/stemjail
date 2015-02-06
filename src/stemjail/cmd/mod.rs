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
use rustc_serialize::json;

pub mod run;

macro_rules! impl_json {
    ($name: ty) => {
        impl $name {
            pub fn decode(encoded: &String) -> json::DecodeResult<$name> {
                json::decode(encoded.as_slice())
            }
            pub fn encode(&self) -> Result<String, json::EncoderError> {
                json::encode(&self)
            }
        }
    }
}

pub trait KageCommand {
    fn get_name<'a>(&'a self) -> &'a String;
    fn get_usage(&self) -> String;
    fn call(&mut self, args: &Vec<String>) -> Result<(), String>;
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub enum PortalCall {
    Run(run::RunAction)
}
impl_json!(PortalCall);

#[derive(Copy, RustcDecodable, RustcEncodable, Show)]
pub struct PortalAck {
    pub request: PortalRequest,
}
impl_json!(PortalAck);

#[derive(Copy, RustcDecodable, RustcEncodable, Show)]
pub enum PortalRequest {
    Nop,
    CreateTty,
}

fn list_kage_cmds<'a>() -> Vec<Box<KageCommand + 'a>> {
    vec!(
        Box::new(self::run::RunKageCmd::new()) as Box<KageCommand>,
    )
}

pub fn get_kage_cmd(name: &String) -> Option<Box<KageCommand>> {
    for cmd in list_kage_cmds().into_iter() {
        if cmd.get_name() == name {
            return Some(cmd);
        }
    }
    None
}

pub fn list_kage_cmd_names() -> Vec<String> {
    list_kage_cmds().iter().map(|x| x.get_name().clone()).collect()
}
