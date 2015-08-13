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

#![feature(collections)]
#![feature(convert)]
#![feature(core)]
#![feature(io)]
#![feature(libc)]
#![feature(path_ext)]
#![feature(path_relative_from)]
#![feature(rustc_private)]
#![feature(std_misc)]
#![feature(unsafe_destructor)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;

extern crate bufstream;
extern crate fd;
extern crate getopts;
extern crate graphviz;
extern crate libc;
extern crate mnt;
extern crate rand;
extern crate rustc_serialize;
extern crate stemflow;
extern crate toml;
extern crate tty;
extern crate unix_socket;

pub mod cmd;
pub mod config;
pub mod fdpass;
pub mod jail;
pub mod srv;
pub mod util;

#[macro_use]
mod ffi;

pub static PORTAL_SOCKET_PATH: &'static str = "./portal.sock";
pub static PORTAL_PROFILES_PATH: &'static str = "./config/profiles";

pub static MONITOR_SOCKET_PATH: &'static str = "/tmp/monitor.sock";

// Wait 100 milliseconds
pub static EVENT_TIMEOUT: Option<u64> = Some(100);
