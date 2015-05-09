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

/// Finite-state machine for a `KageCommand` call

use cmd::MonitorCall;
use cmd::util::send;
use MONITOR_SOCKET_PATH;
use std::marker::PhantomData;
use std::old_io::BufferedStream;
use std::old_io::net::pipe::UnixStream;
use std::old_path::posix::Path as OldPath;
use super::{MountAction, MountRequest};

// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
}

pub struct KageFsm<T> {
    stream: UnixStream,
    _state: PhantomData<T>,
}

// Dummy FSM for now, but help to keep it consistent and enforce number of actions
impl KageFsm<state::Init> {
    pub fn new() -> Result<KageFsm<state::Init>, String> {
        let server = OldPath::new(MONITOR_SOCKET_PATH);
        let stream = match UnixStream::connect(&server) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to connect: {}", e)),
        };
        Ok(KageFsm {
            stream: stream,
            _state: PhantomData,
        })
    }

    pub fn send_mount(self, req: MountRequest) -> Result<(), String> {
        let action = MonitorCall::Mount(MountAction::DoMount(req));
        let mut bstream = BufferedStream::new(self.stream);
        send(&mut bstream, action)
    }
}
