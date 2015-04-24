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

/// Finite-state machine for a `KageCommand` call

extern crate iohandle;
extern crate libc;
extern crate pty;

use cmd::{PortalAck, PortalCall, PortalRequest};
use fdpass;
use PORTAL_SOCKET_PATH;
use self::iohandle::FileDesc;
use self::pty::TtyClient;
use std::io;
use std::marker::PhantomData;
use std::old_io::{Buffer, BufferedStream, Writer};
use std::old_io::net::pipe::UnixStream;
use super::{RunAction, RunRequest};

// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
    #[allow(dead_code)]
    pub struct SendFd;
}

pub struct KageFsm<T> {
    stream: UnixStream,
    _state: PhantomData<T>,
}

macro_rules! fsm_new {
    ($stream: expr) => {
        KageFsm {
            stream: $stream,
            _state: PhantomData,
        }
    }
}

impl KageFsm<state::Init> {
    pub fn new() -> Result<KageFsm<state::Init>, String> {
        let server = PORTAL_SOCKET_PATH;
        let stream = match UnixStream::connect(&server) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to connect to client: {}", e)),
        };
        Ok(fsm_new!(stream))
    }

    pub fn send_run(self, req: RunRequest) -> Result<(KageFsm<state::SendFd>, PortalRequest), String> {
        let RunRequest { stdio, .. } = req;
        let action = PortalCall::Run(RunAction::DoRun(req));
        let encoded = match action.encode() {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to encode command: {}", e)),
        };
        let mut bstream = BufferedStream::new(self.stream);
        match bstream.write_line(encoded.as_ref()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send command: {}", e)),
        }
        match bstream.flush() {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send command (flush): {}", e)),
        }

        // Recv ack and infos (e.g. FD passing)
        let encoded_str = match bstream.read_line() {
            Ok(s) => s,
            Err(e) => return Err(format!("Error reading client: {}", e)),
        };
        let decoded = match PortalAck::decode(&encoded_str) {
            Ok(d) => d,
            Err(e) => return Err(format!("Failed to decode JSON: {:?}", e)),
        };

        // TODO: Remove dup checks
        let valid_req = match decoded.request {
            PortalRequest::Nop => true,
            PortalRequest::CreateTty => stdio,
            //_ => false,
        };
        if !valid_req {
            return Err(format!("Invalid request: {:?}", &decoded.request));
        }
        debug!("Receive {:?}", &decoded.request);

        Ok((fsm_new!(bstream.into_inner()), decoded.request))
    }
}

impl KageFsm<state::SendFd> {
    // Send the template TTY
    pub fn create_tty(mut self) -> Result<io::Result<TtyClient>, String> {
        let peer = FileDesc::new(libc::STDIN_FILENO, false);
        // TODO: Replace &[0] with a JSON command
        let iov = &[0];
        // Block the read stream with a FD barrier
        match fdpass::send_fd(&mut self.stream, iov, &peer) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to synchronise: {}", e)),
        }
        match fdpass::send_fd(&mut self.stream, iov, &peer) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send template FD: {}", e)),
        }

        // Receive the master TTY
        // TODO: Replace 0u8 with a JSON match
        let master = match fdpass::recv_fd(&mut self.stream, vec!(0u8)) {
            Ok(master) => master,
            Err(e) => return Err(format!("Failed to receive master FD: {}", e)),
        };
        Ok(TtyClient::new(master, peer))
    }
}
