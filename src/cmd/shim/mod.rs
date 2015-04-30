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

use getopts::Options;
use jail::{Jail, JailFn, WORKDIR_PARENT};
use jail::util::nest_path;
use rustc_serialize::json;
use self::fsm_kage::KageFsm;
use self::fsm_monitor::MonitorFsmInit;
use std::fmt;
use std::fs;
use std::io;
use std::old_io::BufferedStream;
use std::old_io::net::pipe::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use stemflow::FileAccess;
use super::util;

mod fsm_kage;
mod fsm_monitor;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ShimAction {
    Access(AccessRequest),
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
            ShimAction::Access(req) => {
                match req.check() {
                    Ok(_) => {
                        let bundle = MonitorBundle {
                            request: req,
                            machine: None,
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
    pub fn check(&self) -> Result<(), String> {
        util::check_parent_path(&self.path)
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
    fn call(&mut self, _: &mut Jail) {
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


#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct AccessRequest {
    pub path: PathBuf,
    pub write: bool,
}

impl AccessRequest {
    pub fn check(&self) -> Result<(), String> {
        util::check_parent_path(&self.path)
    }
}

impl JailFn for MonitorBundle<AccessRequest> {
    fn call(&mut self, jail: &mut Jail) {
        debug!("Ask access: {} {}", self.request.path.display(), self.request.write);
        let acl = if self.request.write {
            FileAccess::new_rw(self.request.path.clone())
        } else {
            FileAccess::new_ro(self.request.path.clone())
        };
        match acl {
            Ok(acl) => {
                match jail.gain_access(acl) {
                    Ok(()) => debug!("Access granted"),
                    Err(()) => debug!("Access denied"),
                }
            }
            Err(()) => {
                error!("Failed to create an ACL for {}", self.request.path.display());
            }
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
        opts.optopt("a", "access", "Ask to access a path from the parent", "PATH");
        opts.optflag("w", "write", "Ask for write access");
        ShimKageCmd {
            name: "shim".to_string(),
            opts: opts,
        }
    }

    fn do_list(&self, path: PathBuf) -> Result<(), String> {
        let req = ListRequest {
            path: path,
        };
        try!(req.check());

        let machine = try!(KageFsm::new());
        let machine = try!(machine.send_list_request(req));
        let list = try!(machine.recv_list_response()).result;
        println!("{}", list.into_iter().map(|x| x.to_string_lossy().into_owned()).collect::<Vec<_>>().connect("\n"));
        Ok(())
    }

    fn do_access(&self, path: PathBuf, write: bool) -> Result<(), String> {
        let req = AccessRequest {
            path: path,
            write: write,
        };
        try!(req.check());

        let machine = try!(KageFsm::new());
        try!(machine.send_access_request(req));
        Ok(())
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

        match matches.opt_str("list") {
            Some(path) => {
                check_remaining!(matches);
                return self.do_list(PathBuf::from(path));
            }
            None => {}
        }

        match matches.opt_str("access") {
            Some(path) => {
                check_remaining!(matches);
                return self.do_access(PathBuf::from(path), matches.opt_present("write"));
            }
            None => {}
        }

        Err("No command".into())
    }
}
