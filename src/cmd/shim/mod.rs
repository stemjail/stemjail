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
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use stemflow::{Access, Action, SetAccess, FileAccess};
use super::util;
use unix_socket::UnixStream;

mod fsm_kage;
mod fsm_monitor;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ShimAction {
    Access(AccessRequest),
    List(ListRequest),
}

impl ShimAction {
    pub fn call(self, cmd_tx: Sender<Box<JailFn>>, client: UnixStream) -> Result<(), String> {
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
    // TODO: Remove the Option (need to revamp the JailFn::call() use)
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
    fn call(&mut self, jail: &mut Jail) {
        if jail.is_confined() {
            warn!("Unauthorized command");
            return;
        }
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

/// An `AccessData` always imply at least a read access
#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct AccessData {
    pub path: PathBuf,
    pub write: bool,
}

impl Into<Vec<Arc<FileAccess>>> for AccessData {
    fn into(self) -> Vec<Arc<FileAccess>> {
        let path = Arc::new(self.path);
        let mut ret = vec!(Arc::new(FileAccess {
            path: path.clone(),
            action: Action::Read,
        }));
        if self.write {
            ret.push(Arc::new(FileAccess {
                path: path.clone(),
                action: Action::Write,
            }));
        }
        ret
    }
}

/// There is two caches:
/// * `granted` is used to prune the `access_data` request hierarchy
/// * `denied` is used to find an exact match for the `access_data` request
/// e.g. Deny /var but allow /var/cache
pub struct AccessCache {
    granted: BTreeSet<Arc<FileAccess>>,
    denied: BTreeSet<Arc<FileAccess>>,
}

impl AccessCache {
    pub fn new() -> AccessCache {
        AccessCache {
            granted: BTreeSet::new(),
            denied: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct AccessRequest {
    pub data: AccessData,
    pub get_all_access: bool,
}

impl AccessRequest {
    pub fn check(&self) -> Result<(), String> {
        util::check_parent_path(&self.data.path)
    }

    pub fn new<T>(path: T, write: bool) -> AccessRequest where T: AsRef<Path> {
        AccessRequest {
            data: AccessData {
                path: path.as_ref().to_path_buf(),
                write: write,
            },
            get_all_access: false,
        }
    }
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct AccessResponse {
    pub new_access: Vec<AccessData>,
}
impl_json!(AccessResponse);

impl JailFn for MonitorBundle<AccessRequest> {
    fn call(&mut self, jail: &mut Jail) {
        let acl = if self.request.data.write {
            FileAccess::new_rw(self.request.data.path.clone())
        } else {
            FileAccess::new_ro(self.request.data.path.clone())
        };
        let response = AccessResponse {
            new_access: {
                let ret = match acl {
                    Ok(acl) => {
                        match jail.gain_access(acl) {
                            Ok(new_access) => {
                                debug!("Access granted to {:?}", new_access);
                                new_access
                            }
                            Err(()) => {
                                debug!("Access denied");
                                vec!()
                            }
                        }
                    }
                    Err(()) => {
                        error!("Failed to create an ACL for {:?}", self.request.data);
                        vec!()
                    }
                };
                if self.request.get_all_access {
                    // TODO: Use FileAccess
                    jail.as_ref().binds.iter().map(|x| x.into()).collect()
                } else {
                    ret
                }
            }
        };
        match self.machine.take() {
            Some(m) => {
                match m.send_access_response(response) {
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
        opts.optopt("a", "access", "Ask to access a path from the parent", "PATH");
        opts.optflag("w", "write", "Ask for write access");
        ShimKageCmd {
            name: "shim".to_string(),
            opts: opts,
        }
    }

    pub fn list_directory<T>(path: T) -> Result<(), String> where T: AsRef<Path> {
        let req = ListRequest {
            path: path.as_ref().to_path_buf(),
        };
        try!(req.check());

        let machine = try!(KageFsm::new());
        let machine = try!(machine.send_list_request(req));
        let list = try!(machine.recv_list_response()).result;
        // TODO: Add an output Writer like do_dot()
        println!("{}", list.into_iter().map(|x| x.to_string_lossy().into_owned()).collect::<Vec<_>>().connect("\n"));
        Ok(())
    }

    /// @get_all_access: Get back a list of all current access
    pub fn ask_access(request: AccessRequest) -> Result<Vec<AccessData>, String> {
        try!(request.check());
        let machine = try!(KageFsm::new());
        let machine = try!(machine.send_access_request(request));
        Ok(try!(machine.recv_access_response()).new_access)
    }

    // FIXME: Fake /proc and /dev access authorization
    pub fn cache_ask_access(access_data: AccessData, cache: &mut AccessCache)
            -> Result<(), String> {
        let acl: Vec<Arc<FileAccess>> = access_data.clone().into();
        // The denied cache must exactly match the request to not ignore a valid (nested) one
        if ! acl.iter().find(|&x| ! ( cache.granted.is_allowed(x) || cache.denied.contains(x) )).is_some() {
            Ok(())
        } else {
            let req = AccessRequest {
                data: access_data.clone(),
                get_all_access: cache.granted.is_empty(),
            };

            match ShimKageCmd::ask_access(req) {
                Ok(new_access) => {
                    if new_access.is_empty() {
                        // Access denied
                        for access in acl.into_iter() {
                            let _ = cache.denied.insert(access);
                        }
                    } else {
                        // New access
                        let _ = cache.granted.insert_dedup_all(new_access.into_iter().flat_map(|x| {
                            let i: Vec<Arc<FileAccess>> = x.into();
                            i.into_iter()
                        }));
                    }
                    Ok(())
                }
                Err(e) => {
                    // Cache the request to not replay it
                    for access in acl.into_iter() {
                        let _ = cache.denied.insert(access);
                    }
                    Err(e)
                }
            }
            // TODO: Cleanup included requests if needed (not a big deal because StemJail
            // hints help to get the big picture).
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

        match matches.opt_str("list") {
            Some(path) => {
                check_remaining!(matches);
                return ShimKageCmd::list_directory(PathBuf::from(path));
            }
            None => {}
        }

        match matches.opt_str("access") {
            Some(path) => {
                check_remaining!(matches);
                return match ShimKageCmd::ask_access(
                        AccessRequest::new(path, matches.opt_present("write"))) {
                    Ok(s) => {
                        println!("Gain access: {:?}", s);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            None => {}
        }

        Err("No command".into())
    }
}
