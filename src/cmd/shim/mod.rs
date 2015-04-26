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

extern crate getopts;

use jail::{Jail, JailFn, WORKDIR_PARENT};
use jail::util::nest_path;
use rustc_serialize::json;
use self::fsm_kage::KageFsm;
use self::fsm_monitor::MonitorFsmInit;
use self::getopts::Options;
use std::fmt;
use std::fs;
use std::io;
use std::old_io::BufferedStream;
use std::old_io::net::pipe::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

mod fsm_kage;
mod fsm_monitor;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ShimAction {
    List(ListRequest),
}

impl ShimAction {
    pub fn call(self, cmd_tx: Sender<Box<JailFn>>, client: BufferedStream<UnixStream>) -> Result<(), String> {
        let ret = match self {
            ShimAction::List(req) => {
                match req.check() {
                    Ok(_) => {
                        let bundle = MonitorBundle {
                            request: req,
                            machine: Some(MonitorFsmInit::new(client)),
                        };
                        cmd_tx.send(Box::new(bundle))
                    }
                    Err(e) => return Err(format!("Request error: {}", e)),
                }
            }
        };
        match ret {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to spawn shim action: {}", e)),
        }
    }
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct ListRequest {
    pub path: PathBuf,
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct ListResponse {
    pub result: Vec<PathBuf>,
}
impl_json!(ListResponse);

pub struct MonitorBundle<T> {
    pub request: T,
    pub machine: Option<MonitorFsmInit>,
}

impl<T> fmt::Debug for MonitorBundle<T> where T: fmt::Debug {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "MonitorBundle {{ request: {:?} }}", self.request)
    }
}

impl ListRequest {
    // Forbid use of "." (i.e. the parent domain root directory)
    pub fn check(&self) -> Result<(), String> {
        if ! self.path.is_absolute() {
            return Err("The path is not absolute".to_string());
        }
        // TODO: Factore with jail.import_bind()
        if self.path.starts_with("/proc") {
            return Err("Access denied to /proc".to_string());
        }
        Ok(())
    }
}

impl MonitorBundle<ListRequest> {
    fn list<T>(&self, dir: T) -> io::Result<Vec<PathBuf>> where T: AsRef<Path> {
        let mut ret = vec!();
        for file in try!(fs::read_dir(&dir)) {
            match try!(file).path().relative_from(&dir) {
                Some(d) => ret.push(d.to_path_buf()),
                None => warn!("Failed to get relative path"),
            }
        }
        Ok(ret)
    }
}

impl JailFn for MonitorBundle<ListRequest> {
    // TODO: Spawn a dedicated thread
    fn call(&mut self, _: &Jail) {
        let res = self.list(nest_path(WORKDIR_PARENT, &self.request.path));
        let res = ListResponse {
            result: match res {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to read directory: {}", e);
                    vec!()
                }
            }
        };
        match self.machine.take() {
            Some(m) => {
                match m.send_list_response(res) {
                    Ok(()) => {}
                    Err(e) => error!("Connection result: {:?}", e),
                }
            }
            None => error!("No connection possible"),
        }
    }
}

pub struct ShimKageCmd {
    name: String,
    opts: Options,
}

impl ShimKageCmd {
    pub fn new() -> ShimKageCmd {
        let mut opts = Options::new();
        opts.optflag("h", "help", "Print this message");
        opts.optopt("l", "list", "List a directory from the parent", "DIR");
        ShimKageCmd {
            name: "shim".to_string(),
            opts: opts,
        }
    }
}

macro_rules! get_path {
    ($matches: expr, $name: expr) => {
        match $matches.opt_str($name) {
            Some(s) => PathBuf::from(s),
            None => return Err(format!("Missing {} path", $name)),
        }
    }
}

impl super::KageCommand for ShimKageCmd {
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
        let path = get_path!(matches, "list");

        // Check for remaining useless arguments
        if ! matches.free.is_empty() {
            return Err("Unknown trailing argument".to_string());
        }

        let req = ListRequest {
            path: path,
        };
        match req.check() {
            Ok(_) => {}
            e => return e,
        }

        let machine = try!(KageFsm::new());
        let machine = try!(machine.send_list_request(req));
        let list = try!(machine.recv_list_response()).result;
        println!("{}", list.into_iter().map(|x| x.to_string_lossy().into_owned()).collect::<Vec<_>>().connect("\n"));
        Ok(())
    }
}
