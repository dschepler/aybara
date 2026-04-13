use crate::bindings;
use crate::linux_syscalls::*;
use std::io;
use std::mem;
use std::ptr;

pub struct CloneBuilder {
    args: bindings::clone_args,
}

impl CloneBuilder {
    pub fn new() -> Self {
        Self {
            args: unsafe { mem::zeroed() },
        }
    }

    pub fn generate_signal(mut self) -> Self {
        self.args.exit_signal = bindings::SIGCHLD as u64;
        self
    }

    pub fn new_user_namespace(mut self) -> Self {
        self.args.flags |= bindings::CLONE_NEWUSER as u64;
        self
    }
    pub fn new_pid_namespace(mut self) -> Self {
        self.args.flags |= bindings::CLONE_NEWPID as u64;
        self
    }
    pub fn new_mount_namespace(mut self) -> Self {
        self.args.flags |= bindings::CLONE_NEWNS as u64;
        self
    }
    pub fn new_cgroup_namespace(mut self) -> Self {
        self.args.flags |= bindings::CLONE_NEWCGROUP as u64;
        self
    }
    pub fn new_uts_namespace(mut self) -> Self {
        self.args.flags |= bindings::CLONE_NEWUTS as u64;
        self
    }
    pub fn new_time_namespace(mut self) -> Self {
        self.args.flags |= bindings::CLONE_NEWTIME as u64;
        self
    }
    pub fn new_net_namespace(mut self) -> Self {
        self.args.flags |= bindings::CLONE_NEWNET as u64;
        self
    }

    // Note: once the "never" feature becomes stable, change the
    // type of child_fn to FnOnce() -> !
    pub fn exec<F: FnOnce() -> ()>(mut self, child_fn: F) -> io::Result<ProcessId> {
        unsafe {
            let res = clone3(
                ptr::from_mut(&mut (self.args)),
                mem::size_of_val(&(self.args)),
            );
            if res < 0 {
                Err(io::Error::last_os_error())
            } else if res == 0 {
                child_fn();
                bindings::_exit(0)
            } else {
                Ok(ProcessId(res as bindings::pid_t))
            }
        }
    }
}
