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

use cmd::{PortalAck, PortalCall, PortalRequest};
use cmd::util::{recv, send};
use fdpass;
use libc;
use PORTAL_SOCKET_PATH;
use std::io;
use std::marker::PhantomData;
use super::{RunAction, RunRequest};
use tty::{FileDesc, TtyClient};
use unix_socket::UnixStream;

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
            Err(e) => return Err(format!("Failed to connect: {}", e)),
        };
        Ok(fsm_new!(stream))
    }

    pub fn send_run(mut self, req: RunRequest) -> Result<(KageFsm<state::SendFd>, PortalRequest), String> {
        let stdio = req.stdio;
        let action = PortalCall::Run(RunAction::DoRun(req));
        try!(send(&mut self.stream, action));

        // Recv ack and infos (e.g. FD passing)
        let decoded: PortalAck  = try!(recv(&mut self.stream));

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
        Ok((fsm_new!(self.stream), decoded.request))
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
