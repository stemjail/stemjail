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

#[deriving(Decodable, PartialEq, Show)]
pub struct ProfileConfig {
    pub name: String,
    pub fs: FsConfig,
    pub run: RunConfig,
    pub socket: SocketConfig,
}

#[deriving(Decodable, PartialEq, Show)]
pub struct FsConfig {
    pub root: String,
    pub bind: Option<Vec<BindConfig>>,
}

#[deriving(Decodable, PartialEq, Show)]
pub struct BindConfig {
    pub src: String,
    pub dst: Option<String>,
    pub write: Option<bool>,
}

#[deriving(Decodable, PartialEq, Show)]
pub struct RunConfig {
    pub cmd: Vec<String>,
}

#[deriving(Decodable, PartialEq, Show)]
pub struct SocketConfig {
    pub path: String,
}

#[test]
fn test_get_config_example1() {
    // TODO: Use absolute configuration path
    let c1 = super::get_config::<ProfileConfig>(&Path::new("./config/example1.toml"));
    let c2: Result<ProfileConfig, String> = Ok(ProfileConfig {
        name: "example1".to_string(),
        socket: SocketConfig {
            path: "./portal.sock".to_string(),
        },
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
        },
        run: RunConfig {
            cmd: vec!("/bin/sh".to_string(), "-c".to_string(), "id".to_string()),
        },
    });
    assert!(c1 == c2);
}
