#![allow(dead_code)]

extern crate libc;

use self::libc::c_uint;

bitflags!(
    flags MntFlags: c_uint {
        /* Attempt to forcibily umount */
        const MntForce = 0x00000001,

        /* Just detach from the tree */
        const MntDetach = 0x00000002,

        /* Mark for expiry */
        const MntExpire = 0x00000004,

        /* Don't follow symlink on umount */
        const UmountNofollow = 0x00000008,

        /* Flag guaranteed to be unused */
        const UmountUnused = 0x80000000
    }
)
