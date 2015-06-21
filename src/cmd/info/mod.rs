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

#![allow(deprecated)]

/// `Request::call(&self, PortalFsmInit)` use `PortalFsm`

use getopts::Options;
use rustc_serialize::json;
use self::fsm_kage::KageFsm;
use self::fsm_portal::{PortalFsmInit, PortalFsm};
use srv::{GetDotRequest, ManagerAction};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{Sender, channel};
use unix_socket::UnixStream;

mod fsm_kage;
mod fsm_portal;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum InfoAction {
    GetDot(DotRequest),
}

impl InfoAction {
    pub fn call(&self, stream: UnixStream, manager_tx: Sender<ManagerAction>) -> Result<(), String> {
        match self {
            &InfoAction::GetDot(ref req) => req.call(PortalFsm::new(stream), manager_tx),
        }
    }
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct DotResponse {
    pub result: Option<String>,
}
impl_json!(DotResponse);

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct DotRequest;

impl DotRequest {
    fn call(&self, machine: PortalFsmInit, manager_tx: Sender<ManagerAction>) -> Result<(), String> {
        let (response_tx, response_rx) = channel();
        let action = ManagerAction::GetDot(GetDotRequest {
            response: response_tx,
        });
        // TODO: Add error typing
        match manager_tx.send(action) {
            Ok(()) => {},
            Err(e) => return Err(format!("Failed to send the dot request: {}", e)),
        };
        let dot = match response_rx.recv() {
            Ok(r) => r.dot,
            Err(e) => return Err(format!("Failed to receive the dot response: {}", e)),
        };
        try!(machine.send_dot_response(DotResponse { result: dot }));
        Ok(())
    }
}

pub struct InfoKageCmd {
    name: String,
    opts: Options,
}

impl InfoKageCmd {
    pub fn new() -> InfoKageCmd {
        let mut opts = Options::new();
        opts.optflag("h", "help", "Print this message");
        opts.optflag("d", "dot", "Export the StemFlow graph to DOT");
        opts.optopt("o", "output", "Write the information to a file", "PATH");
        InfoKageCmd {
            name: "info".to_string(),
            opts: opts,
        }
    }

    pub fn do_dot<T>(out: Option<T>) -> Result<(), String> where T: AsRef<Path> {
        let machine = try!(KageFsm::new());
        let machine = try!(machine.send_dot_request(DotRequest));
        let dot = match try!(machine.recv_dot_response()).result {
            Some(dot) => dot,
            None => return Err("Empty graph".into()),
        };
        match out {
            None => {
                println!("{}", dot);
                Ok(())
            }
            Some(path) => match File::create(path) {
                Ok(mut f) => match f.write_all(dot.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => return Err(format!("Failed to write to the file: {}", e)),
                },
                Err(e) => return Err(format!("Failed to open the file: {}", e)),
            }
        }
    }
}

impl super::KageCommand for InfoKageCmd {
    fn get_name<'a>(&'a self) -> &'a String {
        &self.name
    }

    fn get_usage(&self) -> String {
        let msg = format!("Usage for the {} command", self.name);
        format!("{}", self.opts.usage(msg.as_ref()))
    }

    fn call(&mut self, args: &Vec<String>) -> Result<(), String> {
        let matches = match self.opts.parse(args.as_slice()) {
            Ok(m) => m,
            Err(e) => return Err(format!("{}", e)),
        };
        if matches.opt_present("help") {
            println!("{}", self.get_usage());
            return Ok(());
        }

        if matches.opt_present("dot") {
            check_remaining!(matches);
            return InfoKageCmd::do_dot(matches.opt_str("output"));
        }

        Err("No command".into())
    }
}
