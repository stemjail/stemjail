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

extern crate serialize;
extern crate toml;

#[deriving(Decodable, PartialEq, Show)]
pub struct PortalConfig {
    pub name: String,
    pub fs: FsConfig,
    pub run: RunConfig,
    pub socket: SocketConfig,
}

#[deriving(Decodable, PartialEq, Show)]
pub struct FsConfig {
    pub root: String,
}

#[deriving(Decodable, PartialEq, Show)]
pub struct RunConfig {
    pub cmd: Vec<String>,
}

#[deriving(Decodable, PartialEq, Show)]
pub struct SocketConfig {
    pub path: String,
}

pub fn get_config(config_file: &Path) -> Result<PortalConfig, String> {
    let root = match toml::parse_from_file(format!("{}", config_file.display()).as_slice()) {
        Ok(r) => r,
        Err(e) => return Err(format!("Error parsing config file: {}", e)),
    };
    let config: Result<PortalConfig, toml::Error> = toml::from_toml(root);
    match config {
        Ok(c) => Ok(c),
        Err(toml::ParseError) => {
            Err("Parsing error".to_string())
        },
        Err(toml::ParseErrorInField(field)) => {
            Err(format!("Parsing error in field: {}", field))
        },
        Err(toml::IOError(e)) => {
            Err(format!("I/O error: {}", e))
        },
    }
}

#[test]
fn test_get_config_example1() {
    // TODO: Use absolute configuration path
    let c1 = get_config(&Path::new("./config/example1.toml"));
    let c2: Result<PortalConfig, String> = Ok(PortalConfig {
        name: "example1".to_string(),
        socket: SocketConfig { path: "./portal.sock".to_string() },
        fs: FsConfig { root: "./tmp-chroot".to_string() },
        run: RunConfig {
            cmd: vec!("/bin/sh".to_string(), "-c".to_string(), "id".to_string()),
        },
    });
    assert!(c1 == c2);
}
