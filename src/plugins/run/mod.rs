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

extern crate getopts;
extern crate serialize;

use self::getopts::{optflag, getopts, OptGroup};

#[deriving(Clone, Decodable, Encodable, Show)]
pub struct PortalRunCommand {
    pub profile: String,
    pub command: Vec<String>,
}

pub struct RunPlugin {
    name: String,
    opts: Vec<OptGroup>,
    portal_cmd: Option<PortalRunCommand>,
}

impl RunPlugin {
    pub fn new() -> RunPlugin {
        RunPlugin {
            name: "run".to_string(),
            opts: vec!(
                optflag("h", "help", "Print this message"),
            ),
            portal_cmd: None,
        }
    }
}

impl super::Plugin for RunPlugin {
    fn get_name<'a>(&'a self) -> &'a String {
        &self.name
    }

    fn get_portal_cmd(&self) -> Option<super::PortalPluginCommand> {
        match self.portal_cmd {
            Some(ref c) => Some(super::RunCommand(c.clone())),
            None => None,
        }
    }

    fn init_client(&mut self, args: &Vec<String>) -> Result<super::KageAction, String> {
        let matches = match getopts(args.as_slice(), self.opts.as_slice()) {
            Ok(m) => m,
            Err(e) => return Err(format!("{}", e)),
        };
        if matches.opt_present("help") {
            return Ok(super::PrintHelp);
        }
        let mut argi = matches.free.iter();
        let profile = match argi.next() {
            Some(p) => p,
            None => return Err("Need a profile name".to_string()),
        };
        self.portal_cmd = Some(PortalRunCommand {
            profile: profile.clone(),
            command: argi.map(|x| x.to_string()).collect(),
        });
        Ok(super::SendPortalCommand)
    }

    fn get_usage(&self) -> String {
        let msg = format!("Usage for the {} command", self.name);
        format!("{}", getopts::usage(msg.as_slice(), self.opts.as_slice()))
    }
}
