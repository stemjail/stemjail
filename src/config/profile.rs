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

use jail::BindMount;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use stemflow::{Action, Domain, FileAccess, RcDomain, SetAccess};

#[derive(Clone, Debug, RustcDecodable, PartialEq)]
pub struct ProfileConfig {
    pub name: String,
    pub fs: FsConfig,
    pub run: RunConfig,
}

#[derive(Clone, Debug, RustcDecodable, PartialEq)]
pub struct FsConfig {
    pub bind: Option<Vec<BindConfig>>,
}

#[derive(Clone, Debug, RustcDecodable, PartialEq)]
pub struct BindConfig {
    // TODO: Force absolute path
    pub path: String,
    pub write: Option<bool>,
}

#[derive(Clone, Debug, RustcDecodable, PartialEq)]
pub struct RunConfig {
    pub cmd: Vec<String>,
}


impl<'a> Into<Vec<Arc<FileAccess>>> for &'a BindConfig {
    /// Assume there is no relative path, otherwise they are ignored
    fn into(self) -> Vec<Arc<FileAccess>> {
        // TODO: Map between outside/src and inside/dst
        let path = PathBuf::from(self.path.clone());
        // TODO: Put the default policy in unique place
        let file_access = if self.write.unwrap_or(false) {
            FileAccess::new_rw(path)
        } else {
            FileAccess::new_ro(path)
        };
        match file_access {
            Ok(fa) => fa.into_iter().map(|x| Arc::new(x)).collect(),
            Err(()) => vec!(),
        }
    }
}

impl Into<Vec<Arc<FileAccess>>> for ProfileConfig {
    fn into(self) -> Vec<Arc<FileAccess>> {
        match self.fs.bind {
            Some(bind) => bind.into_iter().map(|x| Into::<Vec<Arc<FileAccess>>>::into(&x))
                .flat_map(|x| x.into_iter()).collect(),
            None => vec!(),
        }
    }
}

pub struct ProfileDom {
    pub name: String,
    pub cmd: Vec<String>,
    pub jdom: JailDom,
}

#[derive(Clone)]
pub struct JailDom {
    pub binds: Vec<BindMount>,
    pub dom: Arc<Domain>,
}

impl From<Arc<Domain>> for JailDom {
    /// Loosely conversion: merge read and write into read-write, ignore write-only)
    fn from(other: Arc<Domain>) -> JailDom {
        // TODO: Remove unwrap
        let cwd = env::current_dir().unwrap();
        // For each read access, if the path match a write access, then RW, else RO
        let binds = other.acl.range_read().map(|access_read| {
            let access_write = FileAccess::new(access_read.path.clone(), Action::Write).unwrap();
            let path = cwd.join(access_read.as_ref());
            let mut bind = BindMount::new(path.clone(), path);
            bind.write = other.is_allowed(&Arc::new(access_write));
            bind
        }).collect();
        JailDom {
            binds: binds,
            dom: other,
        }
    }
}


#[test]
fn test_get_config_example1() {
    // TODO: Use absolute configuration path
    let c1: ProfileConfig = match super::get_config("./config/profiles/example1.toml") {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };
    let c2 = ProfileConfig {
        name: "example1".to_string(),
        fs: FsConfig {
            bind: Some(vec!(
                BindConfig {
                    path: "/tmp".to_string(),
                    write: Some(true),
                },
            )),
        },
        run: RunConfig {
            cmd: vec!("/bin/sh".to_string(), "-c".to_string(), "id".to_string()),
        },
    };
    assert_eq!(c1, c2);
}

#[test]
fn test_get_config_example2() {
    // TODO: Use absolute configuration path
    let c1: ProfileConfig = match super::get_config("./config/profiles/example2.toml") {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };
    let c2 = ProfileConfig {
        name: "example2".to_string(),
        fs: FsConfig {
            bind: Some(vec!(
                BindConfig {
                    path: "/run".to_string(),
                    write: Some(true),
                },
                BindConfig {
                    path: "/tmp".to_string(),
                    write: Some(true),
                },
            )),
        },
        run: RunConfig {
            cmd: vec!("/usr/bin/setsid".to_string(), "-c".to_string(), "/bin/sh".to_string()),
        },
    };
    assert_eq!(c1, c2);
}
