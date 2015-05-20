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

#![cfg(not(test))]

#![feature(exit_status)]

extern crate env_logger;
extern crate getopts;
#[macro_use]
extern crate log;
extern crate stemjail;

use getopts::Options;
use std::env;
use stemjail::config::get_configs;
use stemjail::config::portal::Portal;
use stemjail::srv::portal_listen;

macro_rules! exit_error {
    ($($arg:tt)*) => {
        {
            format!($($arg)*);
            env::set_exit_status(1);
            return;
        }
    };
}

fn main() {
    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this message");
    opts.optflag("u", "unconfined", "Enable unconfined features (i.e. allow to access outside the jail) for test purpose");

    let args: Vec<_> = env::args().collect();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!(format!("{}", e)),
    };
    if matches.opt_present("help") {
        println!("{}", opts.usage("Usage: portal [options]"));
        return;
    }

    env_logger::init().unwrap();

    // TODO: Add dynamic configuration reload
    let portal = Portal::new(
        match get_configs(stemjail::PORTAL_PROFILES_PATH) {
            Ok(c) => c,
            Err(e) => exit_error!("Failed to get configuration: {}", e),
        },
        ! matches.opt_present("unconfined"),
    );
    info!("Loaded configuration: {}", portal);
    match portal_listen(portal) {
        Ok(_) => {},
        Err(e) => exit_error!("Failed to listen for clients: {}", e),
    }
}
