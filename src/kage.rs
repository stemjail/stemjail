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

#![feature(phase)]

extern crate iohandle;
extern crate libc;
#[macro_use]
extern crate log;
extern crate stemjail;
extern crate pty;
extern crate serialize;

use stemjail::{fdpass, plugins};
use stemjail::plugins::{PortalRequest, KageAction};
use pty::TtyClient;
use serialize::json;
use std::io::BufferedStream;
use iohandle::FileDesc;
use std::io::net::pipe::UnixStream;
use std::{io, os};

fn get_usage() -> String {
    let default = "stemjail-cli".to_string();
    let args = os::args();
    let name = args.iter().next().unwrap_or(&default);
    format!("usage: {} {}", name, plugins::get_plugins_name().connect("|"))
}

fn args_fail<T: Str>(msg: T) {
    let msg = format!("{}\n\n{}\n", msg.as_slice(), get_usage().as_slice());
    io::stderr().write_str(msg.as_slice()).unwrap();
    os::set_exit_status(1);
}

fn plugin_action(plugin: Box<plugins::Plugin>, cmd: KageAction) -> Result<(), String> {
    match cmd {
        KageAction::Nop => {}
        KageAction::PrintHelp => {
            println!("{}\n{}", plugin.get_usage(), get_usage());
        }
        KageAction::SendPortalCommand => {
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
                Err(e) => return Err(format!("Fail to decode JSON: {:?}", e)),
            };
            if ! cmd.is_valid_request(&decoded.request) {
                return Err(format!("Invalid request: {:?}", &decoded.request));
            }
            debug!("Receive {:?}", &decoded.request);

            // TODO: match decoded.result
            let stream = bstream.unwrap();
            match decoded.request {
                PortalRequest::Nop => {}
                PortalRequest::CreateTty => {
                    // Send the template TTY
                    let peer = FileDesc::new(libc::STDIN_FILENO, false);
                    // TODO: Replace &[0] with a JSON command
                    let iov = &[0];
                    // Block the read stream with a FD barrier
                    match fdpass::send_fd(&stream, iov, &peer) {
                        Ok(_) => {},
                        Err(e) => return Err(format!("Fail to synchronise: {}", e)),
                    }
                    match fdpass::send_fd(&stream, iov, &peer) {
                        Ok(_) => {},
                        Err(e) => return Err(format!("Fail to send template FD: {}", e)),
                    }

                    // Receive the master TTY
                    // TODO: Replace 0u8 with a JSON match
                    let master = match fdpass::recv_fd(&stream, vec!(0u8)) {
                        Ok(master) => master,
                        Err(e) => return Err(format!("Fail to receive master FD: {}", e)),
                    };
                    match TtyClient::new(master, peer) {
                        Ok(p) => p.wait(),
                        Err(e) => panic!("Fail create TTY client: {}", e),
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
                    args_fail("No command with this name");
                    return;
                }
            };
            match plugin.init_client(&plugin_args) {
                Ok(cmd) => match plugin_action(plugin, cmd) {
                    Ok(_) => {
                        // TODO: Wait for the portal ack if PortalRequest::CreateTty
                    }
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
            args_fail("No command");
            return;
        }
    }
}
