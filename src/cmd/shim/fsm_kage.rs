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

#![allow(deprecated)]

/// Finite-state machine for a `KageCommand` call

use cmd::MonitorCall;
use MONITOR_SOCKET_PATH;
use std::marker::PhantomData;
use std::old_io::{Buffer, BufferedStream, Writer};
use std::old_io::net::pipe::UnixStream;
use std::old_path::posix::Path as OldPath;
use super::{AccessRequest, ListRequest, ListResponse, ShimAction};

macro_rules! fsm_next {
    ($myself: expr) => {
        KageFsm {
            bstream: $myself.bstream,
            _state: PhantomData,
        }
    }
}


// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
    #[allow(dead_code)]
    pub struct RecvList;
}

pub struct KageFsm<T> {
    bstream: BufferedStream<UnixStream>,
    _state: PhantomData<T>,
}

// Dummy FSM for now, but help to keep it consistent and enforce number of actions
impl KageFsm<state::Init> {
    pub fn new() -> Result<KageFsm<state::Init>, String> {
        let server = OldPath::new(MONITOR_SOCKET_PATH);
        let bstream = match UnixStream::connect(&server) {
            Ok(s) => BufferedStream::new(s),
            Err(e) => return Err(format!("Failed to connect to client: {}", e)),
        };
        Ok(KageFsm {
            bstream: bstream,
            _state: PhantomData,
        })
    }

    pub fn send_list_request(mut self, req: ListRequest) -> Result<KageFsm<state::RecvList>, String> {
        let action = MonitorCall::Shim(ShimAction::List(req));
        let encoded = match action.encode() {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to encode command: {}", e)),
        };
        match self.bstream.write_line(encoded.as_ref()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send command: {}", e)),
        }
        match self.bstream.flush() {
            Ok(_) => Ok(fsm_next!(self)),
            Err(e) => Err(format!("Failed to flush command: {}", e)),
        }
    }

    pub fn send_access_request(mut self, req: AccessRequest) -> Result<(), String> {
        let action = MonitorCall::Shim(ShimAction::Access(req));
        let encoded = match action.encode() {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to encode command: {}", e)),
        };
        match self.bstream.write_line(encoded.as_ref()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send command: {}", e)),
        }
        match self.bstream.flush() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to flush command: {}", e)),
        }
    }
}

impl KageFsm<state::RecvList> {
    pub fn recv_list_response(mut self) -> Result<ListResponse, String> {
        let encoded_str = match self.bstream.read_line() {
            Ok(s) => s,
            Err(e) => return Err(format!("Error reading client: {}", e)),
        };
        match ListResponse::decode(&encoded_str) {
            Ok(d) => Ok(d),
            Err(e) => return Err(format!("Failed to decode JSON: {:?}", e)),
        }
    }
}
