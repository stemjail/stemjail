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

extern crate getopts;

use self::getopts::{optflag, optopt, getopts, OptGroup};
use std::old_io::BufferedStream;
use std::old_io::net::pipe::UnixStream;
use super::MonitorCall;
use super::super::jail::{BindMount, Jail, JailFn};

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub enum MountAction {
    DoMount(MountRequest),
    //DoUnmount(MountRequest),
}

#[derive(Clone, RustcDecodable, RustcEncodable, Debug)]
pub struct MountRequest {
    pub bind: BindMount,
}

impl JailFn for MountRequest {
    fn call(&self, jail: &Jail) {
        let ret = jail.add_bind(&self.bind, true);
        // TODO: Send result to client
        debug!("Mount result: {:?}", ret);
    }
}

pub struct MountKageCmd {
    name: String,
    opts: Vec<OptGroup>,
}

impl MountKageCmd {
    pub fn new() -> MountKageCmd {
        MountKageCmd {
            name: "mount".to_string(),
            opts: vec!(
                optflag("h", "help", "Print this message"),
                optopt("s", "source", "Set the source path", "SRC"),
                optopt("d", "destination", "Set the destination", "DST"),
                optflag("w", "write", "Set the bind mount writable"),
            ),
        }
    }
}

macro_rules! get_path {
    ($matches: expr, $name: expr) => {
        match $matches.opt_str($name) {
            Some(s) => match Path::new_opt(s) {
                Some(s) => s,
                None => return Err(format!("Bad {} path", $name)),
            },
            None => return Err(format!("Missing {} path", $name)),
        }
    }
}

impl super::KageCommand for MountKageCmd {
    fn get_name<'a>(&'a self) -> &'a String {
        &self.name
    }

    fn get_usage(&self) -> String {
        let msg = format!("Usage for the {} command", self.name);
        format!("{}", getopts::usage(msg.as_slice(), self.opts.as_slice()))
    }

    fn call(&mut self, args: &Vec<String>) -> Result<(), String> {
        let matches = match getopts(args.as_slice(), self.opts.as_slice()) {
            Ok(m) => m,
            Err(e) => return Err(format!("{}", e)),
        };
        if matches.opt_present("help") {
            //println!("{}\n{}", self.get_usage(), get_usage());
            println!("{}", self.get_usage());
            return Ok(());
        }
        let src = get_path!(matches, "source");
        let dst = get_path!(matches, "destination");

        // Check for remaining useless arguments
        if ! matches.free.is_empty() {
            return Err("Unknown trailing argument".to_string());
        }

        let action = MonitorCall::Mount(MountAction::DoMount(MountRequest {
            bind: BindMount {
                src: src,
                dst: dst,
                write: matches.opt_present("write"),
            }
        }));

        let encoded = match action.encode() {
            Ok(s) => s,
            Err(e) => return Err(format!("Fail to encode command: {}", e)),
        };
        let server = Path::new(super::super::MONITOR_SOCKET_PATH);
        let stream = match UnixStream::connect(&server) {
            Ok(s) => s,
            Err(e) => return Err(format!("Fail to connect to client: {}", e)),
        };
        let mut bstream = BufferedStream::new(stream);
        match bstream.write_line(encoded.as_slice()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Fail to send command: {}", e)),
        }
        match bstream.flush() {
            Ok(_) => {},
            Err(e) => return Err(format!("Fail to send command (flush): {}", e)),
        }
        Ok(())
        // TODO: Add ack
    }
}
