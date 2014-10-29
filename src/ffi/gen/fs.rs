#![allow(dead_code)]

extern crate libc;

use self::libc::c_ulong;

bitflags!(
    flags MsFlags: c_ulong {
        /* Mount read-only */
        const MsRdonly = 1,

        /* Ignore suid and sgid bits */
        const MsNosuid = 2,

        /* Disallow access to device special files */
        const MsNodev = 4,

        /* Disallow program execution */
        const MsNoexec = 8,

        /* Writes are synced at once */
        const MsSynchronous = 16,

        /* Alter flags of a mounted FS */
        const MsRemount = 32,

        /* Allow mandatory locks on an FS */
        const MsMandlock = 64,

        /* Directory modifications are synchronous */
        const MsDirsync = 128,

        /* Do not update access times. */
        const MsNoatime = 1024,

        /* Do not update directory access times */
        const MsNodiratime = 2048,

        const MsBind = 4096,

        const MsMove = 8192,

        const MsRec = 16384,

        const MsVerbose = 32768,

        const MsSilent = 32768,

        /* VFS does not apply the umask */
        const MsPosixacl = (1<<16),

        /* change to unbindable */
        const MsUnbindable = (1<<17),

        /* change to private */
        const MsPrivate = (1<<18),

        /* change to slave */
        const MsSlave = (1<<19),

        /* change to shared */
        const MsShared = (1<<20),

        /* Update atime relative to mtime/ctime. */
        const MsRelatime = (1<<21),

        /* this is a kern_mount call */
        const MsKernmount = (1<<22),

        /* Update inode I_version field */
        const MsIVersion = (1<<23),

        /* Always perform atime updates */
        const MsStrictatime = (1<<24),

        const MsNosec = (1<<28),

        const MsBorn = (1<<29),

        const MsActive = (1<<30),

        const MsNouser = (1<<31),

        const MsRmtMask = (MsRdonly.bits|MsSynchronous.bits|MsMandlock.bits|MsIVersion.bits),

        const MsMgcVal = 0xC0ED0000,

        const MsMgcMsk = 0xffff0000
    }
)
