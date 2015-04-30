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

use config::portal::Portal;
use config::profile::ProfileConfig;
use std::sync::mpsc::{Receiver, Sender};

pub enum ManagerAction {
    NewDom(String),
}

pub struct ManagerCall {
    pub action: ManagerAction,
    pub response: Sender<Option<ProfileConfig>>,
}

pub fn manager_listen(portal: Portal, manager_rx: Receiver<ManagerCall>) {
    'listen: loop {
        match manager_rx.recv() {
            Ok(req) => {
                match req.action {
                    ManagerAction::NewDom(name) => {
                        let config = portal.profile(name).cloned();
                        // Do not block
                        match req.response.send(config) {
                            Ok(()) => {}
                            Err(_) => break 'listen,
                        }
                    }
                }
            }
            Err(_) => break 'listen,
        }
    }
}
