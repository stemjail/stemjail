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

use std::marker::PhantomData;
use std::old_io::{BufferedStream, Writer};
use std::old_io::net::pipe::UnixStream;
use super::ListResponse;

// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
}

pub type MonitorFsmInit = MonitorFsm<state::Init>;

struct MonitorFsm<T> {
    stream: BufferedStream<UnixStream>,
    _state: PhantomData<T>,
}

// Dummy FSM for now, but help to keep it consistent and enforce number of actions
impl MonitorFsm<state::Init> {
    pub fn new(stream: BufferedStream<UnixStream>) -> MonitorFsm<state::Init> {
        MonitorFsm {
            stream: stream,
            _state: PhantomData,
        }
    }

    pub fn send_list_response(self, response: ListResponse) -> Result<(), String> {
        let encoded = match response.encode() {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to encode response: {}", e)),
        };
        let mut bstream = BufferedStream::new(self.stream);
        match bstream.write_line(encoded.as_ref()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to send response: {}", e)),
        }
        match bstream.flush() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to flush response: {}", e)),
        }
    }
}
