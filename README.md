# How to enable user namespaces with a Debian kernel?

	# echo 1 > /proc/sys/kernel/unprivileged_userns_clone

OR

	# setcap cap_sys_admin=ep ./target/portal

# How to enable user namespaces with grsecurity?

	# setcap cap_sys_admin,cap_setuid,cap_setgid,cap_dac_override=ep ./target/portal
