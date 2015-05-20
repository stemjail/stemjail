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
use std::io;
use std::sync::mpsc::{Receiver, Sender};

pub enum ManagerAction {
    NewDom(NewDomRequest),
    GetDot(GetDotRequest),
}

pub struct NewDomResponse {
    pub profile: Option<ProfileDom>,
    pub confined: bool,
}

pub struct NewDomRequest {
    pub name: String,
    pub response: Sender<NewDomResponse>,
}

impl NewDomRequest {
    fn call(self, portal: &mut Portal) -> Result<(), ()> {
        let cmd = {
            match portal.profile(&self.name) {
                Some(config) => Some(config.run.cmd.clone()),
                None => None,
            }
        };
        let msg = match cmd {
            Some(cmd) => {
                match portal.domain(&self.name) {
                    Some(jdom) => Some(ProfileDom {
                        name: self.name,
                        cmd: cmd,
                        jdom: jdom.into(),
                    }),
                    None => {
                        error!("No domain found for the config: {:?}", self.name);
                        None
                    }
                }
            }
            None => None,
        };
        // Do not block
        match self.response.send(NewDomResponse { profile: msg, confined: portal.is_confined() }) {
            Ok(()) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub struct GetDotResponse {
    pub dot: Option<String>,
}

pub struct GetDotRequest {
    pub response: Sender<GetDotResponse>,
}

impl GetDotRequest {
    fn call(self, portal: &mut Portal) -> Result<(), ()> {
        let mut dot = io::Cursor::new(vec!());
        let dot = match portal.render(&mut dot) {
            Ok(()) => match String::from_utf8(dot.into_inner()) {
                Ok(s) => Some(s),
                Err(e) => {
                    error!("Failed to convert DOT to UTF8: {}", e);
                    None
                }
            },
            Err(e) => {
                error!("Failed to convert to DOT: {}", e);
                None
            }
        };
        // Do not block
        match self.response.send(GetDotResponse { dot: dot }) {
            Ok(()) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub fn manager_listen(mut portal: Portal, manager_rx: Receiver<ManagerAction>) {
    'listen: loop {
        match manager_rx.recv() {
            Ok(req) => {
                let ret = match req {
                    ManagerAction::NewDom(req) => req.call(&mut portal),
                    ManagerAction::GetDot(req) => req.call(&mut portal),
                };
                if ret.is_err() {
                    break 'listen;
                }
            }
            Err(_) => break 'listen,
        }
    }
}
