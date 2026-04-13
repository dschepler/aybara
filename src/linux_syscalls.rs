use crate::bindings;
use std::ffi::{CString, NulError};
use std::io;
use std::ops::Bound::*;
use std::ops::RangeBounds;
use std::os::fd::RawFd;
use std::os::raw::*;
use std::ptr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UserId(pub bindings::uid_t);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GroupId(pub bindings::gid_t);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProcessId(pub bindings::pid_t);

pub fn geteuid() -> UserId {
    let uid = unsafe { bindings::geteuid() };
    UserId(uid)
}

pub fn getegid() -> GroupId {
    let gid = unsafe { bindings::getegid() };
    GroupId(gid)
}

pub fn set_cloexec<R: RangeBounds<RawFd>>(fd_range: R) -> io::Result<()> {
    let min_fd = match fd_range.start_bound() {
        Included(a) => *a,
        Excluded(a) => a + 1,
        Unbounded => 3 as RawFd,
    };
    let max_fd = match fd_range.end_bound() {
        Included(b) => *b,
        Excluded(b) => b - 1,
        Unbounded => RawFd::MAX,
    };
    if min_fd < (3 as RawFd) {
        Err(io::Error::from(io::ErrorKind::InvalidInput))
    } else {
        let res = unsafe {
            bindings::close_range(
                min_fd as c_uint,
                max_fd as c_uint,
                bindings::CLOSE_RANGE_CLOEXEC as c_int,
            )
        };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

pub fn waitpid(pid: ProcessId) -> io::Result<()> {
    unsafe {
        let res = bindings::waitpid(pid.0, ptr::null_mut(), 0);
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

pub fn chdir(path: &str) -> io::Result<()> {
    let path_c = CString::new(path)?;
    unsafe {
        let res = bindings::chdir(path_c.as_ptr());
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

pub fn execve<'a, T1: IntoIterator<Item: AsRef<str>>, T2: IntoIterator<Item: AsRef<str>>>(
    path: &'a str,
    argv: T1,
    envp: T2,
) -> io::Error {
    // Note: Once the "never" feature becomes stable, can replace the below with:
    // let Err(res) = (|| -> io::Result<!>) { ... })(); res
    (|| -> io::Result<()> {
        let path_c = CString::new(path)?;
        let argv_backing = argv
            .into_iter()
            .map(|s| CString::new(s.as_ref()))
            .collect::<Result<Vec<CString>, NulError>>()?;
        let argv_c = argv_backing
            .iter()
            .map(|s| s.as_ptr() as *mut c_char)
            .chain(Some(ptr::null_mut()))
            .collect::<Vec<*mut c_char>>();
        let envp_backing = envp
            .into_iter()
            .map(|s| CString::new(s.as_ref()))
            .collect::<Result<Vec<CString>, NulError>>()?;
        let envp_c = envp_backing
            .iter()
            .map(|s| s.as_ptr() as *mut c_char)
            .chain(Some(ptr::null_mut()))
            .collect::<Vec<*mut c_char>>();
        unsafe {
            bindings::execve(path_c.as_ptr(), argv_c.as_ptr(), envp_c.as_ptr());
            Err(io::Error::last_os_error())
        }
    })()
    .err()
    .expect("Inner function should never be able to return Ok")
}

pub unsafe fn clone3(cl_args: *mut bindings::clone_args, size: usize) -> c_long {
    unsafe { bindings::syscall(bindings::SYS_clone3 as c_long, cl_args, size) as c_long }
}
pub unsafe fn open_tree_attr(
    dirfd: c_int,
    path: *const c_char,
    flags: c_uint,
    attr: *mut bindings::mount_attr,
    size: usize,
) -> c_int {
    unsafe {
        bindings::syscall(
            bindings::SYS_open_tree_attr as c_long,
            dirfd,
            path,
            flags,
            attr,
            size,
        ) as c_int
    }
}
pub unsafe fn pivot_root(new_root: *const c_char, put_old: *const c_char) -> c_int {
    unsafe { bindings::syscall(bindings::SYS_pivot_root as c_long, new_root, put_old) as c_int }
}
