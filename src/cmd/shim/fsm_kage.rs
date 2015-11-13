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

/// Finite-state machine for a `KageCommand` call

use cmd::MonitorCall;
use cmd::util::{recv, send};
use MONITOR_SOCKET_PATH;
use std::marker::PhantomData;
use super::{AccessRequest, AccessResponse, ListRequest, ListResponse, ShimAction};
use unix_socket::UnixStream;

macro_rules! fsm_next {
    ($myself: expr) => {
        KageFsm {
            stream: $myself.stream,
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
    #[allow(dead_code)]
    pub struct RecvAcl;
}

pub struct KageFsm<T> {
    stream: UnixStream,
    _state: PhantomData<T>,
}

impl KageFsm<state::Init> {
    pub fn new() -> Result<KageFsm<state::Init>, String> {
        let stream = match UnixStream::connect(MONITOR_SOCKET_PATH) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to connect: {}", e)),
        };
        Ok(KageFsm {
            stream: stream,
            _state: PhantomData,
        })
    }

    pub fn send_list_request(mut self, req: ListRequest)
            -> Result<KageFsm<state::RecvList>, String> {
        let action = MonitorCall::Shim(ShimAction::List(req));
        try!(send(&mut self.stream, action));
        Ok(fsm_next!(self))
    }

    pub fn send_access_request(mut self, req: AccessRequest)
            -> Result<KageFsm<state::RecvAcl>, String> {
        let action = MonitorCall::Shim(ShimAction::Access(req));
        try!(send(&mut self.stream, action));
        Ok(fsm_next!(self))
    }
}

impl KageFsm<state::RecvList> {
    pub fn recv_list_response(mut self) -> Result<ListResponse, String> {
        recv(&mut self.stream)
    }
}

impl KageFsm<state::RecvAcl> {
    pub fn recv_access_response(mut self) -> Result<AccessResponse, String> {
        recv(&mut self.stream)
    }
}
