#!/usr/bin/env python3
# -*- coding: utf-8 -*-

# Copyright (C) 2014 Mickaël Salaün
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Lesser General Public License as published by
# the Free Software Foundation, version 3 of the License.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Lesser General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.

import re
import sys

LINUX_SRC = "./linux"

def usage():
    name = sys.argv[0]
    print("usage: {0} [linux-src-dir]".format(name))

def get_header(defbinds):
    ret = "#![allow(dead_code)]\n\nextern crate libc;\n\n"
    for defbind in defbinds:
        ret += "use self::libc::{0};\n".format(defbind.ctype)
    return ret

def to_camel(data):
    return "".join(x.capitalize() for x in data.lower().split("_"))

def to_bits(prefix, data):
    re_name = re.compile(r"({0})_\w+".format(prefix))
    def sub_name(match):
        return "{0}.bits".format(to_camel(match.group()))
    return re_name.sub(sub_name, data)

def gen_flags(define, output, defbinds):
    try:
        with open(define, "r") as fin:
            with open(output, "w") as fout:
                print("Generating {0} FFI…".format(define))
                fout.write(get_header(defbinds))
                for defbind in defbinds:
                    fout.write("\nbitflags!(\n    flags {0}Flags: {1} {{\n".format(to_camel(defbind.name), defbind.ctype))
                    re_define = re.compile(r"^#define\s+(?P<name>(:?{0})_\w+)\s+(?P<value>\S+)\s*(?P<comment>/\*.+?\*/)?.*".format(defbind.prefix))
                    first_time = True
                    for line in fin:
                        match = re_define.match(line)
                        if match:
                            if first_time:
                                first_time = False
                            else:
                                fout.write(",\n\n")
                            comment = match.group("comment")
                            if comment:
                                fout.write("        {0}\n".format(comment))
                            fout.write("        static {0} = {1}".format(to_camel(match.group("name")), to_bits(defbind.prefix, match.group("value"))))
                    fout.write("\n    }\n)\n")
                    fin.seek(0)
    except FileNotFoundError as e:
        print("File not found: {0}\n".format(e))
        usage()
        sys.exit(1)

class DefBind(object):
    def __init__(self, prefix, ctype):
        self.prefix = prefix
        self.ctype = ctype
        self._re_name = re.compile(r"^\w+")

    @property
    def name(self):
        return self._re_name.match(self.prefix).group()

def main(argv):
    src = LINUX_SRC
    if len(argv) > 1:
        src = argv[1]
    include = "{0}/include".format(src)

    defbinds = [DefBind("CLONE", "c_uint")]
    gen_flags("{0}/uapi/linux/sched.h".format(include), "gen/sched.rs", defbinds)

if __name__ == '__main__':
    main(sys.argv)
