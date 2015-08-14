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

use cmd::util::send;
use std::marker::PhantomData;
use super::{AccessResponse, ListResponse};
use unix_socket::UnixStream;

// Private states
mod state {
    #[allow(dead_code)]
    pub struct Init;
}

pub type MonitorFsmInit = MonitorFsm<state::Init>;

struct MonitorFsm<T> {
    stream: UnixStream,
    _state: PhantomData<T>,
}

// Dummy FSM for now, but help to keep it consistent and enforce number of actions
impl MonitorFsm<state::Init> {
    pub fn new(stream: UnixStream) -> MonitorFsm<state::Init> {
        MonitorFsm {
            stream: stream,
            _state: PhantomData,
        }
    }

    pub fn send_list_response(mut self, response: ListResponse) -> Result<(), String> {
        send(&mut self.stream, response)
    }

    pub fn send_access_response(mut self, response: AccessResponse) -> Result<(), String> {
        send(&mut self.stream, response)
    }
}
