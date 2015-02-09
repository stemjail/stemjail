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

use std::old_io::BufferedStream;
use std::old_io::net::pipe::UnixStream;
use super::{MountAction, MountRequest};
use super::super::MonitorCall;
use super::super::super::MONITOR_SOCKET_PATH;

// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
}

pub struct KageFsm<T> {
    stream: UnixStream,
}

// Dummy FSM for now, but help to keep it consistent and enforce number of actions
impl KageFsm<state::Init> {
    pub fn new() -> Result<KageFsm<state::Init>, String> {
        let server = Path::new(MONITOR_SOCKET_PATH);
        let stream = match UnixStream::connect(&server) {
            Ok(s) => s,
            Err(e) => return Err(format!("Fail to connect to client: {}", e)),
        };
        Ok(KageFsm {
            stream: stream,
        })
    }

    pub fn send_mount(self, req: MountRequest) -> Result<(), String> {
        let action = MonitorCall::Mount(MountAction::DoMount(req));
        let encoded = match action.encode() {
            Ok(s) => s,
            Err(e) => return Err(format!("Fail to encode command: {}", e)),
        };
        let mut bstream = BufferedStream::new(self.stream);
        match bstream.write_line(encoded.as_slice()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Fail to send command: {}", e)),
        }
        match bstream.flush() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Fail to send command (flush): {}", e)),
        }
    }
}
