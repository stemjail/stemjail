#![allow(dead_code)]

extern crate libc;

use self::libc::c_uint;

bitflags!(
    flags CloneFlags: c_uint {
        /* set if VM shared between processes */
        const CloneVm = 0x00000100,

        /* set if fs info shared between processes */
        const CloneFs = 0x00000200,

        /* set if open files shared between processes */
        const CloneFiles = 0x00000400,

        /* set if signal handlers and blocked signals shared */
        const CloneSighand = 0x00000800,

        /* set if we want to let tracing continue on the child too */
        const ClonePtrace = 0x00002000,

        /* set if the parent wants the child to wake it up on mm_release */
        const CloneVfork = 0x00004000,

        /* set if we want to have the same parent as the cloner */
        const CloneParent = 0x00008000,

        /* Same thread group? */
        const CloneThread = 0x00010000,

        /* New namespace group? */
        const CloneNewns = 0x00020000,

        /* share system V SEM_UNDO semantics */
        const CloneSysvsem = 0x00040000,

        /* create a new TLS for the child */
        const CloneSettls = 0x00080000,

        /* set the TID in the parent */
        const CloneParentSettid = 0x00100000,

        /* clear the TID in the child */
        const CloneChildCleartid = 0x00200000,

        /* Unused, ignored */
        const CloneDetached = 0x00400000,

        /* set if the tracing process can't force CLONE_PTRACE on this clone */
        const CloneUntraced = 0x00800000,

        /* set the TID in the child */
        const CloneChildSettid = 0x01000000,

        /* New utsname group? */
        const CloneNewuts = 0x04000000,

        /* New ipcs */
        const CloneNewipc = 0x08000000,

        /* New user namespace */
        const CloneNewuser = 0x10000000,

        /* New pid namespace */
        const CloneNewpid = 0x20000000,

        /* New network namespace */
        const CloneNewnet = 0x40000000,

        /* Clone io context */
        const CloneIo = 0x80000000
    }
)
