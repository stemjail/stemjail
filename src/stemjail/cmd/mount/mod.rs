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

use self::fsm_kage::KageFsm;
use self::getopts::{optflag, optopt, getopts, OptGroup};
use std::sync::mpsc::Sender;
use super::super::jail::{BindMount, Jail, JailFn};

mod fsm_kage;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum MountAction {
    DoMount(MountRequest),
    //DoUnmount(MountRequest),
}

impl MountAction {
    pub fn call(self, cmd_tx: Sender<Box<JailFn>>) -> Result<(), String> {
        let ret = match self {
            MountAction::DoMount(req) => {
                match req.check() {
                    Ok(_) => cmd_tx.send(Box::new(req)),
                    Err(e) => return Err(format!("Request error: {}", e)),
                }
            }
        };
        match ret {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Fail to spawn mount action: {}", e)),
        }
    }
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct MountRequest {
    pub bind: BindMount,
}

impl MountRequest {
    // Forbid use of "." (i.e. the parent domain root directory)
    pub fn check(&self) -> Result<(), String> {
        if !self.bind.src.is_absolute() {
            return Err("The mount source is not an absolute path".to_string());
        }
        if !self.bind.dst.is_absolute() {
            return Err("The mount destination is not an absolute path".to_string());
        }
        // FIXME: Add domain transition check (cf. parent mount)
        Ok(())
    }
}

impl JailFn for MountRequest {
    fn call(&self, jail: &Jail) {
        let ret = jail.import_bind(&self.bind);
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
                optopt("d", "destination", "Set the destination path", "DST"),
                optflag("w", "write", "Set the bind mount writable"),
                optflag("p", "parent", "Get the source from the parent domain"),
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

        let req = MountRequest {
            bind: {
                let mut bind = BindMount::new(src, dst);
                bind.write = matches.opt_present("write");
                bind.from_parent = matches.opt_present("parent");
                bind
            }
        };
        match req.check() {
            Ok(_) => {}
            e => return e,
        }

        let machine = try!(KageFsm::new());
        machine.send_mount(req)

        // TODO: Add ack
    }
}
