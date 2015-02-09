// Copyright (C) 2014 Mickaël Salaün
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

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct ProfileConfig {
    pub name: String,
    pub fs: FsConfig,
    pub run: RunConfig,
}

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct FsConfig {
    pub root: String,
    pub bind: Option<Vec<BindConfig>>,
    pub tmp: Option<Vec<TmpConfig>>,
}

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct BindConfig {
    pub src: String,
    pub dst: Option<String>,
    pub write: Option<bool>,
}

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct TmpConfig {
    pub dir: String,
}

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct RunConfig {
    pub cmd: Vec<String>,
}

#[test]
fn test_get_config_example1() {
    // TODO: Use absolute configuration path
    let c1 = match super::get_config::<ProfileConfig>(&Path::new("./config/profiles/example1.toml")) {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };
    let c2 = ProfileConfig {
        name: "example1".to_string(),
        fs: FsConfig {
            root: "./tmp-chroot".to_string(),
            bind: Some(vec!(
                BindConfig {
                    src: "/tmp".to_string(),
                    dst: None,
                    write: Some(true),
                },
                BindConfig {
                    src: "/home".to_string(),
                    dst: Some("/data-ro".to_string()),
                    write: None,
                },
            )),
            tmp: None,
        },
        run: RunConfig {
            cmd: vec!("/bin/sh".to_string(), "-c".to_string(), "id".to_string()),
        },
    };
    assert!(c1 == c2);
}

#[test]
fn test_get_config_example2() {
    // TODO: Use absolute configuration path
    let c1 = match super::get_config::<ProfileConfig>(&Path::new("./config/profiles/example2.toml")) {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };
    let c2 = ProfileConfig {
        name: "example2".to_string(),
        fs: FsConfig {
            root: "/".to_string(),
            bind: Some(vec!(
                BindConfig {
                    src: "/dev/shm".to_string(),
                    dst: Some("/run".to_string()),
                    write: Some(true),
                },
            )),
            tmp: Some(vec!(
                TmpConfig {
                    dir: "/tmp".to_string(),
                },
            )),
        },
        run: RunConfig {
            cmd: vec!("/usr/bin/setsid".to_string(), "-c".to_string(), "/bin/sh".to_string()),
        },
    };
    assert!(c1 == c2);
}
