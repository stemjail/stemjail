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

use std::old_io::{fs, Acceptor, BufferedStream, IoErrorKind, Listener};
use std::old_io::net::pipe::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::Sender;
use std::thread;
use super::cmd::{MonitorCall, PortalCall};
use super::config::portal::Portal;
use super::{EVENT_TIMEOUT, MONITOR_SOCKET_PATH, PORTAL_SOCKET_PATH};
use super::jail::JailFn;

fn read_stream(stream: UnixStream) -> Result<(BufferedStream<UnixStream>, String), String> {
    let mut bstream = BufferedStream::new(stream);
    let encoded = match bstream.read_line() {
        Ok(s) => s,
        Err(e) => return Err(format!("Fail to read command: {}", e)),
    };
    match bstream.flush() {
        Ok(_) => {},
        Err(e) => return Err(format!("Fail to read command (flush): {}", e)),
    };
    Ok((bstream, encoded))
}

fn portal_handle(stream: UnixStream, portal: &Portal) -> Result<(), String> {
    let (bstream, decoded_str) = try!(read_stream(stream));
    let decoded = match PortalCall::decode(&decoded_str) {
        Ok(d) => d,
        Err(e) => return Err(format!("Fail to decode command: {:?}", e)),
    };
    let stream = bstream.into_inner();

    // Use the client command if any or the configuration command otherwise
    match decoded {
        PortalCall::Run(action) => action.call(stream, portal),
    }
}

// TODO: Handle return error
fn monitor_handle(stream: UnixStream, cmd_tx: Sender<Box<JailFn>>) -> Result<(), String> {
    let (_, decoded_str) = try!(read_stream(stream));
    let decoded = match MonitorCall::decode(&decoded_str) {
        Ok(d) => d,
        Err(e) => return Err(format!("Fail to decode command: {:?}", e)),
    };
    match decoded {
        MonitorCall::Mount(action) => action.call(cmd_tx),
    }
}

pub fn portal_listen(portal: Arc<Portal>) -> Result<(), String> {
    let server = Path::new(PORTAL_SOCKET_PATH);
    // FIXME: Use libc::SO_REUSEADDR for unix socket instead of removing the file
    let _ = fs::unlink(&server);
    let stream = UnixListener::bind(&server);
    for stream in stream.listen().incoming() {
        match stream {
            Ok(s) => {
                let portal = portal.clone();
                // TODO: Join all threads
                thread::spawn(move || {
                    match portal_handle(s, &*portal) {
                        Ok(_) => {},
                        Err(e) => println!("Error handling client: {}", e),
                    }
                });
            }
            Err(e) => return Err(format!("Connection error: {}", e)),
        }
    }
    Ok(())
}

// TODO: Handle return error
pub fn monitor_listen(cmd_tx: Sender<Box<JailFn>>, quit: Arc<AtomicBool>) {
    let server = Path::new(MONITOR_SOCKET_PATH);
    // FIXME: Use libc::SO_REUSEADDR for unix socket instead of removing the file
    let _ = fs::unlink(&server);
    let mut acceptor = match UnixListener::bind(&server).listen() {
        Err(e) => {
            warn!("Fail to listen to {:?}: {}", server, e);
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
                thread::spawn(move || {
                    // TODO: Forward the quit event to monitor_handle
                    match monitor_handle(s, client_cmd_fn) {
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
