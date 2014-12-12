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

#![crate_name = "stemjail"]
#![crate_type = "lib"]
#![desc = "stemjail library"]
#![license = "LGPL-3.0"]

#![feature(macro_rules)]
#![feature(phase)]

#[phase(plugin, link)]
extern crate log;
extern crate serialize;

mod macros;

#[path = "../plugins/mod.rs"]
pub mod plugins;

pub mod config;
pub mod fdpass;
pub mod jail;

pub static PORTAL_SOCKET_PATH: &'static str = "./portal.sock";
pub static PORTAL_PROFILES_PATH: &'static str = "./config/profiles";
