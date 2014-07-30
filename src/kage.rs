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

extern crate libc;
extern crate stemjail;
extern crate serialize;

use stemjail::{fdpass, plugins};
use self::native::io::file::FileDesc;
use serialize::json;
use std::io::BufferedStream;
use std::io::net::unix::UnixStream;
use std::{io, os};

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
        plugins::KageNop => {}
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
            let stream = match UnixStream::connect(&server) {
                Ok(s) => s,
                Err(e) => return Err(format!("Fail to connect to client: {}", e)),
            };
            let mut bstream = BufferedStream::new(stream);
            match bstream.write_line(json.as_slice()) {
                Ok(_) => {},
                Err(e) => return Err(format!("Fail to send command: {}", e)),
            }
            match bstream.flush() {
                Ok(_) => {},
                Err(e) => return Err(format!("Fail to send command (flush): {}", e)),
            }

            // Recv ack and infos (e.g. FD passing)
            let encoded_str = match bstream.read_line() {
                Ok(s) => s,
                Err(e) => return Err(format!("Error reading client: {}", e)),
            };
            let decoded: plugins::PortalAck = match json::decode(encoded_str.as_slice()) {
                Ok(d) => d,
                Err(e) => return Err(e.to_string()),
            };
            // TODO: match decoded.result
            let stream = bstream.unwrap();
            match decoded.request {
                plugins::PortalNop => {}
                plugins::RequestFileDescriptor => {
                    let fd = FileDesc::new(libc::STDIN_FILENO, false);
                    // TODO: Replace &[0] with a JSON command
                    let iov = &[0];
                    // Block the read stream with a FD barrier
                    match fdpass::send_fd(&stream, iov, &fd) {
                        Ok(_) => {},
                        Err(e) => return Err(format!("Fail to synchronise: {}", e)),
                    }
                    match fdpass::send_fd(&stream, iov, &fd) {
                        Ok(_) => {},
                        Err(e) => return Err(format!("Fail to send stdio FD: {}", e)),
                    }
                }
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
