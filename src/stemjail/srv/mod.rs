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

use rustc_serialize::json;
use std::old_io::{Acceptor, BufferedStream, IoErrorKind, Listener};
use std::old_io::fs;
use std::old_io::net::pipe::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::Sender;
use std::thread::Thread;
use super::cmd::MonitorCall;
use super::jail::JailFn;
use super::{EVENT_TIMEOUT, MONITOR_SOCKET_PATH};

// TODO: Handle return error
fn handle_cmd(stream: UnixStream, cmd_tx: Sender<Box<JailFn>>) -> Result<(), String> {
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
    let decoded: MonitorCall = match json::decode(encoded_str.as_slice()) {
        Ok(d) => d,
        Err(e) => return Err(format!("Fail to decode command: {:?}", e)),
    };
    match decoded {
        MonitorCall::Mount(action) => action.call(cmd_tx),
    }
}

// TODO: Handle return error
pub fn listen_cmd(cmd_tx: Sender<Box<JailFn>>, quit: Arc<AtomicBool>) {
    let server = Path::new(MONITOR_SOCKET_PATH);
    // FIXME: Use libc::SO_REUSEADDR for unix socket instead of removing the file
    let _ = fs::unlink(&server);
    let mut acceptor = match UnixListener::bind(&server).listen() {
        Err(e) => {
            debug!("Fail to listen to {:?}: {}", server, e);
            return;
        }
        Ok(v) => v,
    };
    while !quit.load(Relaxed) {
        acceptor.set_timeout(EVENT_TIMEOUT);
        match acceptor.accept() {
            Ok(s) => {
                let client_cmd_fn = cmd_tx.clone();
                // TODO: Join all threads
                let _ = Thread::scoped(move || {
                    // TODO: Forward the quit event to handle_cmd
                    match handle_cmd(s, client_cmd_fn) {
                        Ok(_) => {},
                        Err(e) => debug!("Error handling client: {}", e),
                    }
                });
            }
            Err(ref e) if e.kind == IoErrorKind::TimedOut => {}
            Err(e) => debug!("Connection error: {}", e),
        }
    }
}
