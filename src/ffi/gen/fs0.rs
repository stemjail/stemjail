#![allow(dead_code)]

extern crate libc;

use self::libc::c_uint;

bitflags!(
    flags MntFlags: c_uint {
        /* Attempt to forcibily umount */
        const MNT_FORCE = 0x00000001,

        /* Just detach from the tree */
        const MNT_DETACH = 0x00000002,

        /* Mark for expiry */
        const MNT_EXPIRE = 0x00000004,

        /* Don't follow symlink on umount */
        const UMOUNT_NOFOLLOW = 0x00000008,

        /* Flag guaranteed to be unused */
        const UMOUNT_UNUSED = 0x80000000
    }
)
