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

#![crate_name = "portal"]
#![crate_type = "bin"]
#![desc = "stemjail Portal"]
#![license = "LGPL-3.0"]

#![feature(macro_rules)]

extern crate stemjail;
extern crate serialize;

use stemjail::config::get_config;
use stemjail::config::profile::ProfileConfig;
use stemjail::{fdpass, jail, plugins};
use serialize::json;
use std::io::{BufferedStream, Listener, Acceptor};
use std::io::fs;
use std::io::net::pipe::{UnixListener, UnixStream};
use std::os;
use std::sync::Arc;

macro_rules! absolute_path(
    ($dir: expr) => {
        os::getcwd().join(&Path::new($dir))
    };
)

fn handle_client(stream: UnixStream, config: Arc<ProfileConfig>) -> Result<(), String> {
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
    let decoded: plugins::PortalPluginCommand = match json::decode(encoded_str.as_slice()) {
        Ok(d) => d,
        Err(e) => return Err(format!("Fail to decode command: {}", e)),
    };

    // Use the client command if any or the configuration command otherwise
    let (args, do_stdio) = match decoded {
        plugins::RunCommand(r) => {
            let c = match r.command.iter().next() {
                Some(_) => r.command.clone(),
                None => config.run.cmd.clone(),
            };
            (c, r.stdio)
        },
        //_ => config.run.cmd,
    };
    let exe = match args.iter().next() {
        Some(c) => c.clone(),
        None => return Err("Missing executable in the command (first argument)".to_string()),
    };

    let mut j = jail::Jail::new(
        config.name.clone(),
        absolute_path!(config.fs.root.clone()),
        match config.fs.bind {
            Some(ref b) => b.iter().map(
                |x| jail::BindMount {
                    src: absolute_path!(x.src.clone()),
                    dst: match x.dst {
                        Some(ref d) => absolute_path!(d.clone()),
                        None => absolute_path!(x.src.clone()),
                    },
                    write: match x.write {
                        Some(w) => w,
                        None => false,
                    },
                }).collect(),
            None => Vec::new(),
        });

    let cmd = plugins::PortalAck {
        //result: Ok(()),
        request: if do_stdio {
            plugins::RequestFileDescriptor
        } else {
            plugins::PortalNop
        }
    };
    let json = json::encode(&cmd);
    match bstream.write_line(json.as_slice()) {
        Ok(_) => {},
        Err(e) => return Err(format!("Fail to send acknowledgement: {}", e)),
    }
    let stream = bstream.unwrap();

    let stdio = if do_stdio {
        // TODO: Replace 0u8 with a JSON match
        let fd = match fdpass::recv_fd(&stream, vec!(0u8)) {
            Ok(fd) => fd,
            Err(e) => return Err(format!("Fail to receive stdio FD: {}", e)),
        };
        match jail::Stdio::new(fd) {
            Ok(f) => Some(f),
            Err(e) => fail!("Fail create stdio: {}", e),
        }
    } else {
        None
    };

    // Safe tail
    let args = args.iter().enumerate().filter_map(
        |(i, x)| if i == 0 { None } else { Some(x.clone()) } ).collect();

    j.run(&Path::new(exe), &args, &stdio);
    // TODO: Send ACK
    Ok(())
}

macro_rules! exit_error(
    ($($arg:tt)*) => {
        {
            format_args!(::std::io::stdio::println_args, $($arg)*);
            std::os::set_exit_status(1);
            return;
        }
    };
)

fn main() {
    let config = match get_config::<ProfileConfig>(&Path::new(stemjail::PORTAL_PROFILE_PATH)) {
        Ok(c) => Arc::new(c),
        Err(e) => exit_error!("Configuration error: {}", e),
    };
    let server = Path::new(config.socket.path.clone());
    // FIXME: Use libc::SO_REUSEADDR for unix socket instead of removing the file
    let _ = fs::unlink(&server);
    let stream = UnixListener::bind(&server);
    for stream in stream.listen().incoming() {
        match stream {
            Ok(s) => {
                let config = config.clone();
                spawn(proc() {
                    match handle_client(s, config) {
                        Ok(_) => {},
                        Err(e) => println!("Error handling client: {}", e),
                    }
                });
            }
            Err(e) => exit_error!("Connection error: {}", e),
        }
    }
}
