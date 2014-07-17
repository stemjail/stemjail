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

use std::{io, os};

mod plugins;

fn get_usage() -> String {
    let name: String = os::args().shift().unwrap_or("stemjail-cli".to_string());
    let plugins: Vec<String> = plugins::get_plugins().iter().map(|x| x.name.to_string()).collect();
    format!("usage: {} {}", name, plugins.connect("|"))
}

fn args_fail(msg: String) {
    io::stderr().write_line(msg.append("\n").as_slice()).unwrap();
    io::stderr().write_line(get_usage().as_slice()).unwrap();
    os::set_exit_status(1);
}

fn main() {
    let args = os::args().clone();
    let mut args = args.iter();

    let _ = args.next();
    match args.next() {
        Some(cmd) => {
            let plugin_args: Vec<String> = args.map(|x| x.to_string()).collect();
            match plugins::command(cmd, &plugin_args) {
                Ok(_) => {}
                Err(e) => args_fail(format!("Command error: {}", e)),
            }
        }
        None => args_fail("No command".to_string()),
    }
}
