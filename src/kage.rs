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

#![crate_name = "kage"]
#![crate_type = "bin"]
#![desc = "stemjail CLI"]
#![license = "LGPL-3.0"]

// Need deriving Decodable and Encodable
extern crate serialize;

use std::io::net::unix::UnixStream;
use std::{io, os};
use self::serialize::json;

mod stemjail;
mod plugins;

fn get_usage() -> String {
    let name: String = os::args().shift().unwrap_or("stemjail-cli".to_string());
    format!("usage: {} {}", name, plugins::get_plugins_name().connect("|"))
}

fn args_fail(msg: String) {
    io::stderr().write_line(msg.append("\n").as_slice()).unwrap();
    io::stderr().write_line(get_usage().as_slice()).unwrap();
    os::set_exit_status(1);
}

fn plugin_action(plugin: Box<plugins::Plugin>, cmd: plugins::KageAction) -> Result<(), String> {
    match cmd {
        plugins::Nop => {}
        plugins::PrintHelp => {
            println!("{}\n{}", plugin.get_usage(), get_usage());
        }
        plugins::SendPortalCommand => {
            let cmd = match plugin.get_portal_cmd() {
                Some(c) => c,
                None => return Err("No command".to_string()),
            };
            let json = json::encode(&cmd);
            let server = Path::new(stemjail::PORTAL_SOCKET_PATH);
            let mut stream = UnixStream::connect(&server);
            match stream.write_str(json.as_slice()) {
                Ok(_) => {},
                Err(e) => return Err(format!("{}", e)),
            }
        }
    }
    Ok(())
}

fn main() {
    let args = os::args().clone();
    let mut args = args.iter();

    let _ = args.next();
    match args.next() {
        Some(cmd) => {
            let plugin_args: Vec<String> = args.map(|x| x.to_string()).collect();
            let mut plugin = match plugins::get_plugin(cmd) {
                Some(p) => p,
                None => {
                    args_fail("No command with this name".to_string());
                    return;
                }
            };
            match plugin.init_client(&plugin_args) {
                Ok(cmd) => match plugin_action(plugin, cmd) {
                    Ok(_) => {}
                    Err(e) => {
                        args_fail(format!("Command action error: {}", e));
                        return;
                    }
                },
                Err(e) => {
                    args_fail(format!("Command error: {}", e));
                    return;
                }
            }
        }
        None => {
            args_fail("No command".to_string());
            return;
        }
    }
}
