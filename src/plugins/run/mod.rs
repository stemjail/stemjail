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

use self::getopts::{optflag, getopts, OptGroup};

struct PortalRunCommand {
    profile: String,
    command: Vec<String>,
}

struct RunPlugin {
    name: String,
    opts: Vec<OptGroup>,
}

impl RunPlugin {
    fn new() -> RunPlugin {
        RunPlugin {
            name: "run".to_string(),
            opts: vec!(
                optflag("h", "help", "Print this message"),
            ),
        }
    }
}

impl super::Plugin for RunPlugin {
    fn get_name<'a>(&'a self) -> &'a String {
        &self.name
    }

    fn init_client(&self, args: &Vec<String>) -> Result<super::PluginCommand, String> {
        let matches = match getopts(args.as_slice(), self.opts.as_slice()) {
            Ok(m) => m,
            Err(e) => return Err(format!("{}", e)),
        };
        if matches.opt_present("h") {
            return Ok(super::PrintHelp);
        }
        let mut argi = matches.free.iter();
        let profile = match argi.next() {
            Some(p) => p,
            None => return Err("Need a profile name".to_string()),
        };
        let command = match argi.next() {
            Some(c) => c,
            None => return Err("Need a command".to_string()),
        };
        let portal_cmd = PortalRunCommand {
            profile: profile.clone(),
            command: argi.map(|x| x.to_string()).collect(),
        };
        println!("run profile: {}", profile);
        println!("run args: {}", matches.free);
        Ok(super::Nop)
    }

    fn get_usage(&self) -> String {
        let msg = format!("Usage for the {} command", self.name);
        format!("{}", getopts::usage(msg.as_slice(), self.opts.as_slice()))
    }
}

pub fn get_plugin() -> Box<super::Plugin> {
    box RunPlugin::new() as Box<super::Plugin>
}
