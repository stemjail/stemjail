#![allow(dead_code)]

extern crate libc;

use self::libc::c_ulong;

bitflags!(
    flags MsFlags: c_ulong {
        /* Mount read-only */
        static MsRdonly = 1,

        /* Ignore suid and sgid bits */
        static MsNosuid = 2,

        /* Disallow access to device special files */
        static MsNodev = 4,

        /* Disallow program execution */
        static MsNoexec = 8,

        /* Writes are synced at once */
        static MsSynchronous = 16,

        /* Alter flags of a mounted FS */
        static MsRemount = 32,

        /* Allow mandatory locks on an FS */
        static MsMandlock = 64,

        /* Directory modifications are synchronous */
        static MsDirsync = 128,

        /* Do not update access times. */
        static MsNoatime = 1024,

        /* Do not update directory access times */
        static MsNodiratime = 2048,

        static MsBind = 4096,

        static MsMove = 8192,

        static MsRec = 16384,

        static MsVerbose = 32768,

        static MsSilent = 32768,

        /* VFS does not apply the umask */
        static MsPosixacl = (1<<16),

        /* change to unbindable */
        static MsUnbindable = (1<<17),

        /* change to private */
        static MsPrivate = (1<<18),

        /* change to slave */
        static MsSlave = (1<<19),

        /* change to shared */
        static MsShared = (1<<20),

        /* Update atime relative to mtime/ctime. */
        static MsRelatime = (1<<21),

        /* this is a kern_mount call */
        static MsKernmount = (1<<22),

        /* Update inode I_version field */
        static MsIVersion = (1<<23),

        /* Always perform atime updates */
        static MsStrictatime = (1<<24),

        static MsNosec = (1<<28),

        static MsBorn = (1<<29),

        static MsActive = (1<<30),

        static MsNouser = (1<<31),

        static MsRmtMask = (MsRdonly.bits|MsSynchronous.bits|MsMandlock.bits|MsIVersion.bits),

        static MsMgcVal = 0xC0ED0000,

        static MsMgcMsk = 0xffff0000
    }
)
