#![allow(dead_code)]

extern crate libc;

use self::libc::c_ulong;

bitflags! {
    flags MsFlags: c_ulong {
        /* Mount read-only */
        const MS_RDONLY = 1,

        /* Ignore suid and sgid bits */
        const MS_NOSUID = 2,

        /* Disallow access to device special files */
        const MS_NODEV = 4,

        /* Disallow program execution */
        const MS_NOEXEC = 8,

        /* Writes are synced at once */
        const MS_SYNCHRONOUS = 16,

        /* Alter flags of a mounted FS */
        const MS_REMOUNT = 32,

        /* Allow mandatory locks on an FS */
        const MS_MANDLOCK = 64,

        /* Directory modifications are synchronous */
        const MS_DIRSYNC = 128,

        /* Do not update access times. */
        const MS_NOATIME = 1024,

        /* Do not update directory access times */
        const MS_NODIRATIME = 2048,

        const MS_BIND = 4096,

        const MS_MOVE = 8192,

        const MS_REC = 16384,

        const MS_VERBOSE = 32768,

        const MS_SILENT = 32768,

        /* VFS does not apply the umask */
        const MS_POSIXACL = (1<<16),

        /* change to unbindable */
        const MS_UNBINDABLE = (1<<17),

        /* change to private */
        const MS_PRIVATE = (1<<18),

        /* change to slave */
        const MS_SLAVE = (1<<19),

        /* change to shared */
        const MS_SHARED = (1<<20),

        /* Update atime relative to mtime/ctime. */
        const MS_RELATIME = (1<<21),

        /* this is a kern_mount call */
        const MS_KERNMOUNT = (1<<22),

        /* Update inode I_version field */
        const MS_I_VERSION = (1<<23),

        /* Always perform atime updates */
        const MS_STRICTATIME = (1<<24),

        const MS_NOSEC = (1<<28),

        const MS_BORN = (1<<29),

        const MS_ACTIVE = (1<<30),

        const MS_NOUSER = (1<<31),

        const MS_RMT_MASK = (MS_RDONLY.bits|MS_SYNCHRONOUS.bits|MS_MANDLOCK.bits|MS_I_VERSION.bits),

        const MS_MGC_VAL = 0xC0ED0000,

        const MS_MGC_MSK = 0xffff0000
    }
}
