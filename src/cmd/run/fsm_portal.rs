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

/// Finite-state machine for a `RunRequest` call

use bufstream::BufStream;
use cmd::PortalAck;
use fdpass;
use jail;
use pty::FileDesc;
use std::marker::PhantomData;
use unix_socket::UnixStream;

// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
    #[allow(dead_code)]
    pub struct RecvFd;
    #[allow(dead_code)]
    pub struct SendFd;
}

pub struct RequestFsm<T> {
    stream: UnixStream,
    _state: PhantomData<T>,
}

pub type RequestInit = RequestFsm<state::Init>;

macro_rules! fsm_new {
    ($stream: expr) => {
        RequestFsm {
            stream: $stream,
            _state: PhantomData,
        }
    }
}

macro_rules! fsm_next {
    ($myself: expr) => {
        RequestFsm {
            stream: $myself.stream,
            _state: PhantomData,
        }
    }
}

impl RequestFsm<state::Init> {
    pub fn new(stream: UnixStream) -> RequestFsm<state::Init> {
        fsm_new!(stream)
    }

    pub fn send_ack(self, ack: PortalAck) -> Result<RequestFsm<state::RecvFd>, String>{
        let encoded = match ack.encode() {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to encode command: {}", e)),
        };
        let mut bstream = BufStream::new(self.stream);
        match bstream.write_line(encoded.as_ref()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send acknowledgement: {}", e)),
        }
        Ok(fsm_new!(bstream.into_inner()))
    }
}

impl RequestFsm<state::RecvFd> {
    pub fn recv_fd(mut self) -> Result<(RequestFsm<state::SendFd>, FileDesc), String> {
        // TODO: Replace 0u8 with a JSON match
        let fd = match fdpass::recv_fd(&mut self.stream, vec!(0u8)) {
            Ok(fd) => fd,
            Err(e) => return Err(format!("Failed to receive template FD: {}", e)),
        };
        Ok((fsm_next!(self), fd))
    }

    pub fn no_recv_fd(self) -> RequestFsm<state::SendFd> {
        fsm_next!(self)
    }
}

impl RequestFsm<state::SendFd> {
    pub fn send_fd(mut self, stdio: &jail::Stdio) -> Result<(), String> {
        // TODO: Replace &[0] with a JSON command
        let iov = &[0];
        match fdpass::send_fd(&mut self.stream, iov, stdio.get_master()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to synchronise: {}", e)),
        }
        match fdpass::send_fd(&mut self.stream, iov, stdio.get_master()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send stdio FD: {}", e)),
        }
        Ok(())
    }
}
