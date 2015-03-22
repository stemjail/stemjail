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

#![feature(env)]
#![feature(old_path)]

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate stemjail;

use std::sync::Arc;
use stemjail::config::get_configs;
use stemjail::config::portal::Portal;
use stemjail::config::profile::ProfileConfig;
use stemjail::srv::portal_listen;

macro_rules! exit_error {
    ($($arg:tt)*) => {
        {
            format!($($arg)*);
            std::env::set_exit_status(1);
            return;
        }
    };
}

fn main() {
    env_logger::init().unwrap();

    // TODO: Add dynamic configuration reload
    let portal = Arc::new(Portal {
        configs: match get_configs::<ProfileConfig>(&Path::new(stemjail::PORTAL_PROFILES_PATH)) {
            Ok(c) => c,
            Err(e) => exit_error!("Fail to get configuration: {}", e),
        }
    });
    let names: Vec<&String> = portal.configs.iter().map(|x| &x.name ).collect();
    info!("Loaded configurations: {:?}", names);
    match portal_listen(portal.clone()) {
        Ok(_) => {},
        Err(e) => exit_error!("Fail to listen for clients: {}", e),
    }
}
