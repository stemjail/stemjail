# StemJail: Dynamic user-activity isolation

StemJail brings to the enduser a mechanism to automatically isolate processes according to the data access pattern (usage).
This proof of concept relies on Linux user namespaces and thus is available to unprivileged users.
It is complementary to the usual access control put in place by the administrator and provide another layer of security for the enduser.


## Warning

This project is a work in progress and is not ready for production yet.
Use it at your own risk.
The API and commands may change.

StemJail relies on Linux kernel security features and Rust libraries.
This project takes security seriously and will make efforts to stay secure.


## Evolving jails

A profile configuration reflects a user activity (e.g. bank account, personal pictures...).
The only interactive step involving the enduser is the profile creation.
Profiles are then used by StemJail as domains to create dedicated jails per activity.

A common jail starts with a minimal effective access set but a potentially wide access set.
In a jail, when a user application process tries to access a path, the jail can evolve to a new environment with a wider effective access set but a tighter potential access set.
This way, user activity (and the domain) can automatically be detected without interrupting the user's workflow.
See StemFlow for more details.

Each jail get a dedicated */dev*, */proc* and */tmp* (ephemeral files).


# Architecture overview

StemJail is split into multiple libraries, one per repository:
* *stemjail*: create and manage jails, according to the security policy, with three components: *kage*, *portal* and *monitor*
* *stemflow*: access-control policy engine (domain transition)
* *stemshim*: compatibility shim to preload the StemJail client (*kage*) in processes
* *pty-rs*: create and manage pseudoterminals
* *mnt-rs*: mounts point listing

The three components of StemJail can communicate through UNIX sockets:
```
                   ---------------
                   | Kage client |
                   ---------------
                         ||
                         \/
                  -----------------
                  | Portal daemon |
                  -----------------
                         ||
     ---------------------------------------------
     | jail #1           ||                      |
     |                   \/                      |
     |           ------------------              |
     |           | Monitor daemon |              |
     |           ------------------              |
     |                   /\                      |
     |                   ||                      |
     |  ---------------------------------------  |
     |  | Application client #1 (app. + kage) |  |
     |  ---------------------------------------  |
     |                                           |
     ---------------------------------------------
```

## Kage

Kage is the client part of StemJail.
It has two roles: *portal* client and *monitor* client.

From outside the jail, it can send commands to a *portal* instance:
* *run*: create, connect and launch an application in a new dedicated jail according to the application path or a configuration profile
* *info*: create and download the current domains graph (DOT file format)

From inside the jail, it can send commands to a *monitor* instance:
* *shim*: send access notification/request and list files
* *mount*: mount from inside or outside (disabled in safe mode)


## Portal

When launched, the portal parses the configuration profiles and listens on a UNIX socket for incomming trusted commands from a *kage* client.
Its purpose is to spawn new jails, forward I/O (e.g. terminal) and get informations from its handled jails.


## Monitor

The monitor is a server listening on a UNIX socket for incomming untrusted commands from a *kage* client.
Each jail have it's own monitor, which is its first process.
It sets up the jail's environment (e.g. file system) according to the configuration profiles.
The monitor is the only process able to add more access to its jail.

The monitor checks the jail policy for each new access request.
If an access request is allowed, the monitor transition its jail from the current domain to the one matching the request, if any.
When switching to a new domain, the monitor adds the new access to the jail.
These accesses are translated to bind mounts that expose new file hierarchies from outside the jail.


## User application

The user application process should be loaded in the jail with StemShim which hooks open-like functions.
These hooks notify the monitor of access requests with the client part of StemJail (*kage*).
There is a cache per thread to limit the number of requests for a near-zero performance impact.

It's useless for a malicious process not to notify the monitor because then the jail (and so the process) can't get new access.


# Try StemJail

## Requirements

For now, you need Rust 1.0.0-beta (the 1.0.0 stable support is a work in progress).
To easily build all dependencies, you need Cargo (the 0.2.0 build with Rust 1.0.0-beta).

## Clone repositories

```
$ git clone https://github.com/stemjail/stemjail
$ git clone https://github.com/stemjail/stemflow
$ git clone https://github.com/stemjail/stemshim
$ git clone https://github.com/stemjail/pty-rs
$ git clone https://github.com/stemjail/mnt-rs
$ git clone https://github.com/stemjail/termios.rs --branch static
```

## Build StemShim

```
$ cd stemshim
$ make
```

## Build StemJail

```
$ cd stemjail
$ cargo build
```

## Create profiles

You need to create your profiles in the *config/profiles* directory.
Each profile must specify all ressources needed to run your application (e.g. */usr*, */lib*...).
All paths in your profiles must exist on the filesystem.
Take a look at the examples.


## Run portal

To run portal in debug mode:
```
./tools/portal.sh
```


## Run a jail

For now, we need to manually set the environment to preload StemShim:
```
./tools/kage.sh run -t -- /path/to/stemjail/tools/env.sh /path/to/your/application
```


# FAQ

## How to enable user namespaces with a Debian kernel?

By default (for now), Debian does not activate user namespaces.
You can activate this feature with:

```
# echo 1 > /proc/sys/kernel/unprivileged_userns_clone
```


## How to enable user namespaces with grsecurity?

A process running in a grsecurity patched kernel need `CAP_SYS_ADMIN`, `CAP_SETUID` and `CAP_SETGID` to use user namespaces (cf. *kernel/user_namespace.c:create_user_ns*).
The `CAP_DAC_OVERRIDE` is also needed to write to the */proc/<pid>/{{uid,gid}_map,setgroups}* files.

This procedure is dangerous because it gives too much rights to StemJail which is not yet ready to manage this safely.
For now, use this for test purposes only!

```
# setcap cap_sys_admin,cap_setuid,cap_setgid,cap_dac_override=ep /path/to/target/debug/portal
```


## Which options are needed in my vanilla kernel?

You need to enable all Linux namespaces:
```
CONFIG_NAMESPACES=y
CONFIG_UTS_NS=y
CONFIG_IPC_NS=y
CONFIG_USER_NS=y
CONFIG_PID_NS=y
CONFIG_NET_NS=y
```


## What is the difference between StemJail and LXC, Docker, OpenVZ, VServer or a *chroot*?

StemJail's goal is not to manage virtual machine but to use Linux namespaces to isolate processes according to a simple configuration.
Whereas other solutions use root privileges to do administrative tasks, StemJail only relies on user namespaces, so it's not possible to perform all tasks required for a full system (e.g. create devices, mount whatever you want).
For this reason, if we trust the kernel to isolate properly, then there is no way a jailed (malicious) process can get more rights than the user using StemJail.


## What is the difference between StemJail and SELinux, AppArmor, Tomoyo or grsecurity RBAC?

StemJail's goal is to be used and tuned by unprivileged users to protect their data.
Most mandatory access control (MAC) systems are designed to be configured by an administrator (e.g. root) to protect the system against its users and to protect users between them.
With StemJail we want to protect ourselves (as simple users) against our individual (potentially compromised) processes.
So we want to be able to control what our applications have access to even if we are not an administrator.

Moreover, StemJail should work on most Linux distros without custom kernel nor patched applications!
This way, it's a lot more easier to keep your system updated.
