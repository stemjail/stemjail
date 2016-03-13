#![allow(dead_code)]

extern crate libc;


bitflags! {
    pub flags CloneFlags: ::libc::c_uint {
        /** set if VM shared between processes */
        const CLONE_VM = 0x00000100,

        /** set if fs info shared between processes */
        const CLONE_FS = 0x00000200,

        /** set if open files shared between processes */
        const CLONE_FILES = 0x00000400,

        /** set if signal handlers and blocked signals shared */
        const CLONE_SIGHAND = 0x00000800,

        /** set if we want to let tracing continue on the child too */
        const CLONE_PTRACE = 0x00002000,

        /** set if the parent wants the child to wake it up on mm_release */
        const CLONE_VFORK = 0x00004000,

        /** set if we want to have the same parent as the cloner */
        const CLONE_PARENT = 0x00008000,

        /** Same thread group? */
        const CLONE_THREAD = 0x00010000,

        /** New mount namespace group */
        const CLONE_NEWNS = 0x00020000,

        /** share system V SEM_UNDO semantics */
        const CLONE_SYSVSEM = 0x00040000,

        /** create a new TLS for the child */
        const CLONE_SETTLS = 0x00080000,

        /** set the TID in the parent */
        const CLONE_PARENT_SETTID = 0x00100000,

        /** clear the TID in the child */
        const CLONE_CHILD_CLEARTID = 0x00200000,

        /** Unused, ignored */
        const CLONE_DETACHED = 0x00400000,

        /** set if the tracing process can't force CLONE_PTRACE on this clone */
        const CLONE_UNTRACED = 0x00800000,

        /** set the TID in the child */
        const CLONE_CHILD_SETTID = 0x01000000,

        /** New utsname namespace */
        const CLONE_NEWUTS = 0x04000000,

        /** New ipc namespace */
        const CLONE_NEWIPC = 0x08000000,

        /** New user namespace */
        const CLONE_NEWUSER = 0x10000000,

        /** New pid namespace */
        const CLONE_NEWPID = 0x20000000,

        /** New network namespace */
        const CLONE_NEWNET = 0x40000000,

        /** Clone io context */
        const CLONE_IO = 0x80000000
    }
}
