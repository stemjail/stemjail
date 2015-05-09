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

use cmd::util::send;
use std::marker::PhantomData;
use std::old_io::BufferedStream;
use std::old_io::net::pipe::UnixStream;
use super::DotResponse;

// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
}

pub struct PortalFsm<T> {
    stream: UnixStream,
    _state: PhantomData<T>,
}

pub type PortalFsmInit = PortalFsm<state::Init>;

impl PortalFsm<state::Init> {
    pub fn new(stream: UnixStream) -> PortalFsm<state::Init> {
        PortalFsm {
            stream: stream,
            _state: PhantomData,
        }
    }

    pub fn send_dot_response(self, response: DotResponse) -> Result<(), String> {
        let mut bstream = BufferedStream::new(self.stream);
        try!(send(&mut bstream, response));
        Ok(())
    }
}
