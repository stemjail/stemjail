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
use std::old_io::BufferedStream;
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
            Err(e) => return Err(format!("Failed to connect: {}", e)),
        };
        Ok(KageFsm {
            bstream: bstream,
            _state: PhantomData,
        })
    }

    pub fn send_list_request(mut self, req: ListRequest)
            -> Result<KageFsm<state::RecvList>, String> {
        let action = MonitorCall::Shim(ShimAction::List(req));
        try!(send(&mut self.bstream, action));
        Ok(fsm_next!(self))
    }

    pub fn send_access_request(mut self, req: AccessRequest) -> Result<(), String> {
        let action = MonitorCall::Shim(ShimAction::Access(req));
        try!(send(&mut self.bstream, action));
        Ok(())
    }
}

impl KageFsm<state::RecvList> {
    pub fn recv_list_response(mut self) -> Result<ListResponse, String> {
        recv(&mut self.bstream)
    }
}
