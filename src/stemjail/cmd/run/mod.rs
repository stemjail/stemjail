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

/// `Request::call(&self, RequestInit)` use `RequestFsm`

extern crate getopts;

use self::fsm_kage::KageFsm;
use self::fsm_portal::{RequestInit, RequestFsm};
use self::getopts::{optflag, getopts, OptGroup};
use std::old_io::net::pipe::UnixStream;
use std::os;
use super::{PortalAck, PortalRequest};
use super::super::config::portal::Portal;
use super::super::jail;

mod fsm_kage;
mod fsm_portal;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum RunAction {
    DoRun(RunRequest),
}

impl RunAction {
    pub fn call(&self, stream: UnixStream, portal: &Portal) -> Result<(), String> {
        match self {
            &RunAction::DoRun(ref req) => req.call(RequestFsm::new(stream), portal),
        }
    }
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct RunRequest {
    pub profile: String,
    pub command: Vec<String>,
    pub stdio: bool,
}

// FIXME: Replace Path::new with Path::new_opt
macro_rules! absolute_path {
    ($cwd: expr, $dir: expr) => {
        $cwd.join(&Path::new($dir.clone()))
    };
}

impl RunRequest {
    fn call(&self, machine: RequestInit, portal: &Portal) -> Result<(), String> {
        let config = match portal.configs.iter().find(|c| { c.name == self.profile }) {
            Some(c) => c,
            None => return Err(format!("No profile named `{}`", self.profile)),
        };
        let args = match self.command.iter().next() {
            Some(_) => self.command.clone(),
            None => config.run.cmd.clone(),
        };
        let exe = match args.iter().next() {
            Some(c) => c.clone(),
            None => return Err("Missing executable in the command (first argument)".to_string()),
        };
        let cwd = match os::getcwd() {
            Ok(d) => d,
            Err(e) => return Err(format!("Fail to get CWD: {}", e)),
        };

        let mut j = jail::Jail::new(
            config.name.clone(),
            absolute_path!(cwd, config.fs.root),
            match config.fs.bind {
                Some(ref b) => b.iter().map(
                    |x| {
                        let mut bind = jail::BindMount::new(
                            absolute_path!(cwd, x.src),
                            match x.dst {
                                Some(ref d) => absolute_path!(cwd, d),
                                None => absolute_path!(cwd, x.src),
                            });
                        bind.write = match x.write {
                            Some(w) => w,
                            None => false,
                        };
                        bind
                    }).collect(),
                None => Vec::new(),
            },
            match config.fs.tmp {
                Some(ref b) => b.iter().map(
                    |x| jail::TmpfsMount {
                        name: None,
                        dst: Path::new(&x.dir),
                    }).collect(),
                None => Vec::new(),
            }
        );

        let ack = PortalAck {
            request: if self.stdio {
                PortalRequest::CreateTty
            } else {
                PortalRequest::Nop
            }
        };
        let machine = try!(machine.send_ack(ack));

        let (machine, stdio) = if self.stdio {
            let (machine, fd) = try!(machine.recv_fd());
            // XXX: Allocate a new TTY inside the jail?
            match jail::Stdio::new(&fd) {
                Ok(f) => (machine, Some(f)),
                Err(e) => panic!("Fail create stdio: {}", e),
            }
        } else {
            (machine.no_recv_fd(), None)
        };

        // Safe tail
        let args = args.iter().enumerate().filter_map(
            |(i, x)| if i == 0 { None } else { Some(x.clone()) } ).collect();

        j.run(&Path::new(exe), &args, stdio);
        match j.get_stdio() {
            &Some(ref s) => {
                try!(machine.send_fd(s))
            },
            &None => {}
        };
        debug!("Waiting jail to end");
        let ret = j.wait();
        debug!("Jail end: {:?}", ret);
        Ok(())
    }
}

pub struct RunKageCmd {
    name: String,
    opts: Vec<OptGroup>,
}

impl RunKageCmd {
    pub fn new() -> RunKageCmd {
        RunKageCmd {
            name: "run".to_string(),
            opts: vec!(
                optflag("h", "help", "Print this message"),
                optflag("t", "tty", "Create and connect to the remote TTY"),
            ),
        }
    }
}

impl super::KageCommand for RunKageCmd {
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
            println!("{}", self.get_usage());
            return Ok(());
        }
        let mut argi = matches.free.iter();
        let profile = match argi.next() {
            Some(p) => p,
            None => return Err("Need a profile name".to_string()),
        };
        let stdio = matches.opt_present("tty");
        let req = RunRequest {
            profile: profile.clone(),
            command: argi.map(|x| x.to_string()).collect(),
            stdio: stdio
        };

        let machine = try!(KageFsm::new());
        let (machine, ret) = try!(machine.send_run(req));

        // TODO: match decoded.result
        match ret {
            PortalRequest::Nop => {}
            PortalRequest::CreateTty => {
                match try!(machine.create_tty()) {
                    Ok(p) => p.wait(),
                    Err(e) => panic!("Fail create TTY client: {}", e),
                }
            }
        }
        Ok(())
    }
}
