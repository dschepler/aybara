# aybara

This project aims to provide containers for building packages (for
example, think of it as forming the core of an `sbuild` backend).
These containers are truly rootless, in the sense that the system
administrator does not even need to assign subuid or subgid ranges for
the user, or allow access to the setuid `newuidmap` and `newgidmap`
wrappers.  All that is needed is a Linux kernel with the appropriate
namespace capabilities.

## Design

In order to achieve this, an aybara instance provides two nested
containers.  The outer container simulates root access, and is
intended for installing build dependencies (for example using `apt-get
build-dep`).  The inner container simulates a build user environment,
and should be used for the bulk of the operations.

Within each of these containers, there is a small process filling the
role of an `init` style zombie reaper, and also providing access on an
external Unix domain socket to a process spawning service.
Communication of an outside driver with these processes is handled by
passing pipe file descriptors through the Unix domain socket to be
assigned to stdin/stdout/stderr of the spawned process.

## Limitations

Because of the design of avoiding relying on subuid and subgid ranges,
each container only has a single user.  This means in particular:

* The root container still cannot `chown` or `chgrp` any files to any
  users other than root.
* The build user container still has read-only access to all files in
  the outer root container.  In particular, shadow passwords will not
  work as expected (but then again, you should not be using any
  sensitive passwords in these containers anyway, and optimally you
  should not need any passwords at all).
* In order to prevent user container processes from killing root container
  processes, it is necessary to have the user container be in its own PID
  namespace.  That means that root processes will see different PIDs for
  user container processes than those processes themselves do.
* If the root container requires network access (as opposed to using a
  bind mount of a local repository mirror), then it will have all
  network access that the invoking user does.  It is not possible to
  set up extra firewall rules to restrict access to only certain
  hosts.  This also means that any TCP/IP services you run in the root
  container will also be accessible to other users on the system, and
  depending on the listening mode, even to users on other hosts.
* In the mode where the root container gets network access, TCP/IP
  services you run in the root container will not be accessible to
  processes in the build user container.
