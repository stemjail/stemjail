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
use config::profile::ProfileDom;
use std::sync::mpsc::{Receiver, Sender};

pub enum ManagerAction {
    NewDom(String),
}

pub struct ManagerCall {
    pub action: ManagerAction,
    pub response: Sender<Option<ProfileDom>>,
}

pub fn manager_listen(mut portal: Portal, manager_rx: Receiver<ManagerCall>) {
    'listen: loop {
        match manager_rx.recv() {
            Ok(req) => {
                match req.action {
                    ManagerAction::NewDom(name) => {
                        let cmd = {
                            match portal.profile(&name) {
                                Some(config) => Some(config.run.cmd.clone()),
                                None => None,
                            }
                        };
                        let msg = match cmd {
                            Some(cmd) => {
                                match portal.domain(&name) {
                                    Some(jdom) => Some(ProfileDom {
                                        name: name,
                                        cmd: cmd,
                                        jdom: jdom.into(),
                                    }),
                                    None => {
                                        error!("No domain found for the config: {:?}", name);
                                        None
                                    }
                                }
                            }
                            None => None,
                        };
                        // Do not block
                        match req.response.send(msg) {
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
