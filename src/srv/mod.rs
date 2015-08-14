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

#![allow(deprecated)]

use bufstream::BufStream;
use cmd::{MonitorCall, PortalCall};
use config::portal::Portal;
use {EVENT_TIMEOUT, MONITOR_SOCKET_PATH, PORTAL_SOCKET_PATH};
use jail::JailFn;
use self::manager::manager_listen;
use std::fs;
use std::io::{BufRead, ErrorKind, Write};
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{Sender, channel};
use std::thread;
use unix_socket::{UnixListener, UnixStream};

pub use srv::manager::{DomDesc, GetDotRequest, ManagerAction, NewDomRequest};

mod manager;

fn read_stream(stream: UnixStream) -> Result<(BufStream<UnixStream>, String), String> {
    let mut bstream = BufStream::new(stream);
    let mut encoded = String::new();
    match bstream.read_line(&mut encoded) {
        Ok(_) => {}
        Err(e) => return Err(format!("Failed to read command: {}", e)),
    };
    match bstream.flush() {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to flush command: {}", e)),
    };
    Ok((bstream, encoded))
}

fn portal_handle(stream: UnixStream, manager_tx: Sender<ManagerAction>) -> Result<(), String> {
    let (bstream, decoded_str) = try!(read_stream(stream));
    let decoded = match PortalCall::decode(&decoded_str) {
        Ok(d) => d,
        Err(e) => return Err(format!("Failed to decode command: {:?}", e)),
    };
    let stream = match bstream.into_inner() {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to pass command: {:?}", e)),
    };

    debug!("Portal got request: {:?}", decoded);
    // Use the client command if any or the configuration command otherwise
    match decoded {
        PortalCall::Run(action) => action.call(stream, manager_tx),
        PortalCall::Info(action) => action.call(stream, manager_tx),
    }
}

// TODO: Handle return error
fn monitor_handle(stream: UnixStream, cmd_tx: Sender<Box<JailFn>>) -> Result<(), String> {
    let (bstream, decoded_str) = try!(read_stream(stream));
    let decoded = match MonitorCall::decode(&decoded_str) {
        Ok(d) => d,
        Err(e) => return Err(format!("Failed to decode command: {:?}", e)),
    };
    debug!("Monitor got request: {:?}", decoded);
    match decoded {
        MonitorCall::Mount(action) => action.call(cmd_tx),
        MonitorCall::Shim(action) => action.call(cmd_tx, bstream),
    }
}

fn portal_ext_listen(manager_tx: Sender<ManagerAction>) {
    let server = PORTAL_SOCKET_PATH;
    // FIXME: Use libc::SO_REUSEADDR for unix socket instead of removing the file
    let _ = fs::remove_file(&server);
    let stream = match UnixListener::bind(&server) {
        Ok(s) => s,
        Err(e) => {
            // Can failed because of read-only FS/directory (e.g. no tmpfs for the socket) and then
            // the monitor will fail to receive any command.
            error!("Failed to bind to {:?}: {}", server, e);
            // FIXME: Handle return error instead of exit
            exit(1);
        }
    };
    for client in stream.incoming() {
        match client {
            Ok(c) => {
                let manager_tx = manager_tx.clone();
                // TODO: Join all threads
                thread::spawn(|| {
                    match portal_handle(c, manager_tx) {
                        Ok(_) => {},
                        Err(e) => error!("Error handling client: {}", e),
                    }
                });
            }
            Err(e) => {
                warn!("Portal connection error: {}", e);
                return;
            }
        }
    }
}

pub fn portal_listen(portal: Portal) -> Result<(), String> {
    let (manager_tx, manager_rx) = channel();
    thread::spawn(|| portal_ext_listen(manager_tx));

    // Spawn the domain manager on the current thread
    manager_listen(portal, manager_rx);
    Ok(())
}

// FIXME: Handle return error
pub fn monitor_listen(cmd_tx: Sender<Box<JailFn>>, quit: Arc<AtomicBool>) {
    let server = MONITOR_SOCKET_PATH;
    // FIXME: Use libc::SO_REUSEADDR for unix socket instead of removing the file
    let _ = fs::remove_file(&server);
    let mut acceptor = match UnixListener::bind(&server) {
        Err(e) => {
            // Can failed because of read-only FS/directory (e.g. no tmpfs for the socket) and then
            // the monitor will fail to receive any command.
            error!("Failed to bind to {:?}: {}", server, e);
            // FIXME: Handle return error instead of exit
            exit(1);
        }
        Ok(v) => v,
    };
    while !quit.load(Relaxed) {
        match acceptor.accept() {
            Ok(s) => {
                let client_cmd_fn = cmd_tx.clone();
                // TODO: Join all threads
                thread::spawn(|| {
                    // TODO: Forward the quit event to monitor_handle
                    match monitor_handle(s, client_cmd_fn) {
                        Ok(_) => {},
                        Err(e) => debug!("Error handling client: {}", e),
                    }
                });
            }
            Err(ref e) if e.kind() == ErrorKind::TimedOut => {}
            Err(e) => debug!("Connection error: {}", e),
        }
    }
}
