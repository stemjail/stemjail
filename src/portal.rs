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

#![feature(io)]
#![feature(path)]
#![feature(std_misc)]

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate stemjail;

use std::old_io::{BufferedStream, Listener, Acceptor};
use std::old_io::fs;
use std::old_io::net::pipe::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread::Thread;
use stemjail::cmd::PortalCall;
use stemjail::config::get_configs;
use stemjail::config::portal::Portal;
use stemjail::config::profile::ProfileConfig;

fn handle_client(stream: UnixStream, portal: Arc<Portal>) -> Result<(), String> {
    let mut bstream = BufferedStream::new(stream);
    let encoded_str = match bstream.read_line() {
        Ok(s) => s,
        Err(e) => return Err(format!("Fail to read command: {}", e)),
    };
    match bstream.flush() {
        Ok(_) => {},
        Err(e) => return Err(format!("Fail to read command (flush): {}", e)),
    }
    // FIXME: task '<main>' failed at 'called `Option::unwrap()` on a `None` value', .../rust/src/libcore/option.rs:265
    let decoded = match PortalCall::decode(&encoded_str) {
        Ok(d) => d,
        Err(e) => return Err(format!("Fail to decode command: {:?}", e)),
    };

    // Use the client command if any or the configuration command otherwise
    match decoded {
        PortalCall::Run(action) => action.call(bstream, &portal),
    }
}

macro_rules! exit_error {
    ($($arg:tt)*) => {
        {
            format!($($arg)*);
            std::os::set_exit_status(1);
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
            Err(e) => exit_error!("{}", e),
        }
    });
    let names: Vec<&String> = portal.configs.iter().map(|x| &x.name ).collect();
    info!("Loaded configurations: {:?}", names);
    let server = Path::new(stemjail::PORTAL_SOCKET_PATH);
    // FIXME: Use libc::SO_REUSEADDR for unix socket instead of removing the file
    let _ = fs::unlink(&server);
    let stream = UnixListener::bind(&server);
    for stream in stream.listen().incoming() {
        match stream {
            Ok(s) => {
                let portal = portal.clone();
                Thread::spawn(move || {
                    match handle_client(s, portal) {
                        Ok(_) => {},
                        Err(e) => println!("Error handling client: {}", e),
                    }
                });
            }
            Err(e) => exit_error!("Connection error: {}", e),
        }
    }
}
