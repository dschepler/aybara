use crate::bindings;
use crate::linux_syscalls::*;
use std::ffi::{CString, NulError};
use std::io;
use std::mem;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::os::raw::*;
use std::ptr;

#[allow(dead_code)]
pub enum AtPath<'a> {
    CurrentDir,
    RelativeToCurrentDir(&'a str), // also accepts absolute paths
    ExactFd(BorrowedFd<'a>),
    RelativeToFd(BorrowedFd<'a>, &'a str), // also accepts absolute paths, in which case the BorrowedFd is ignored
    Absolute(&'a str),                     // use if you want errors on relative path
}

pub struct AtPathSyscallArgs {
    pub fd_arg: RawFd,
    pub path_arg: CString,
    pub flags_arg: i32,
}

impl AtPath<'_> {
    pub fn as_syscall_args(&self) -> Result<AtPathSyscallArgs, NulError> {
        Ok(match self {
            Self::CurrentDir => AtPathSyscallArgs {
                fd_arg: bindings::AT_FDCWD as RawFd,
                path_arg: CString::from(c""),
                flags_arg: bindings::AT_EMPTY_PATH as i32,
            },
            Self::ExactFd(fd) => AtPathSyscallArgs {
                fd_arg: fd.as_raw_fd(),
                path_arg: CString::from(c""),
                flags_arg: bindings::AT_EMPTY_PATH as i32,
            },
            Self::RelativeToFd(fd, path) => AtPathSyscallArgs {
                fd_arg: fd.as_raw_fd(),
                path_arg: CString::new(*path)?,
                flags_arg: 0,
            },
            Self::RelativeToCurrentDir(path) => AtPathSyscallArgs {
                fd_arg: bindings::AT_FDCWD as RawFd,
                path_arg: CString::new(*path)?,
                flags_arg: 0,
            },
            Self::Absolute(path) => AtPathSyscallArgs {
                fd_arg: -1 as RawFd,
                path_arg: CString::new(*path)?,
                flags_arg: 0,
            },
        })
    }
    pub fn as_syscall_args_ioerr(&self) -> io::Result<AtPathSyscallArgs> {
        Ok(self.as_syscall_args()?)
    }
}

pub trait AsPathFd: AsFd {
    #[allow(unused)]
    fn as_atpath(&self) -> AtPath<'_> {
        AtPath::ExactFd(self.as_fd())
    }
    fn relpath<'a>(&'a self, path: &'a str) -> AtPath<'a> {
        AtPath::RelativeToFd(self.as_fd(), path)
    }
}

#[repr(u32)]
#[allow(dead_code)]
pub enum MountPropagation {
    NoBind = bindings::MS_UNBINDABLE,
    Private = bindings::MS_PRIVATE,
    Slave = bindings::MS_SLAVE,
    Shared = bindings::MS_SHARED,
}

pub struct MountAttrBuilder {
    attr: bindings::mount_attr,
}

impl MountAttrBuilder {
    pub fn new() -> Self {
        Self {
            attr: unsafe { mem::zeroed() },
        }
    }
    pub fn propagation(mut self, propval: MountPropagation) -> Self {
        self.attr.propagation = propval as u64;
        self
    }
    pub fn build(self) -> bindings::mount_attr {
        self.attr
    }
}

pub struct BindMountBuilder {
    flags: c_uint,
    attr: Option<bindings::mount_attr>,
}

impl BindMountBuilder {
    pub fn new() -> Self {
        Self {
            flags: bindings::OPEN_TREE_CLONE,
            attr: None,
        }
    }
    pub fn recursive(mut self) -> Self {
        self.flags |= bindings::AT_RECURSIVE;
        self
    }
    pub fn cloexec(mut self) -> Self {
        self.flags |= bindings::OPEN_TREE_CLOEXEC;
        self
    }
    pub fn attr(mut self, attrval: bindings::mount_attr) -> Self {
        self.attr = Some(attrval);
        self
    }
    pub fn open(mut self, path: AtPath) -> io::Result<MountFd> {
        let path_args = path.as_syscall_args_ioerr()?;
        self.flags |= path_args.flags_arg as u32;
        unsafe {
            let res = open_tree_attr(
                path_args.fd_arg,
                path_args.path_arg.as_ptr(),
                self.flags,
                self.attr.as_mut().map_or(ptr::null_mut(), ptr::from_mut),
                self.attr.as_ref().map_or(0, mem::size_of_val),
            );
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(MountFd::from_raw_fd(res as RawFd))
            }
        }
    }
}

pub struct FilesystemBuilder {
    flags: c_uint,
}
impl FilesystemBuilder {
    pub fn new() -> Self {
        Self { flags: 0 }
    }
    pub fn cloexec(mut self) -> Self {
        self.flags |= bindings::FSOPEN_CLOEXEC as c_uint;
        self
    }
    pub fn build_config(self, fstype: &str) -> io::Result<FilesystemConfigBuilder> {
        let fstype_c = CString::new(fstype)?;
        unsafe {
            let res = bindings::fsopen(fstype_c.as_ptr(), self.flags);
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(FilesystemConfigBuilder::from_raw_fd(res as RawFd))
            }
        }
    }
}

pub struct FilesystemConfigBuilder {
    config_fd: OwnedFd,
    fsmount_flags: c_uint,
    fsmount_attr_flags: c_uint,
}
impl FromRawFd for FilesystemConfigBuilder {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            config_fd: unsafe { OwnedFd::from_raw_fd(fd) },
            fsmount_flags: 0,
            fsmount_attr_flags: 0,
        }
    }
}
impl FilesystemConfigBuilder {
    pub fn cloexec(mut self) -> Self {
        self.fsmount_flags |= bindings::FSMOUNT_CLOEXEC as c_uint;
        self
    }
    pub fn nodev(mut self) -> Self {
        self.fsmount_attr_flags |= bindings::MOUNT_ATTR_NODEV;
        self
    }
    pub fn nosuid(mut self) -> Self {
        self.fsmount_attr_flags |= bindings::MOUNT_ATTR_NOSUID;
        self
    }
    pub fn noexec(mut self) -> Self {
        self.fsmount_attr_flags |= bindings::MOUNT_ATTR_NOEXEC;
        self
    }
    pub fn set_string(self, key: &str, val: &str) -> io::Result<Self> {
        let key_c = CString::new(key)?;
        let val_c = CString::new(val)?;
        unsafe {
            let res = bindings::fsconfig(
                self.config_fd.as_raw_fd() as c_int,
                bindings::fsconfig_command_FSCONFIG_SET_STRING as c_uint,
                key_c.as_ptr(),
                val_c.as_ptr() as *const c_void,
                0,
            );
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(self)
            }
        }
    }
    fn fsconfig_create(self) -> io::Result<Self> {
        unsafe {
            let res = bindings::fsconfig(
                self.config_fd.as_raw_fd() as c_int,
                bindings::fsconfig_command_FSCONFIG_CMD_CREATE as c_uint,
                ptr::null(),
                ptr::null(),
                0,
            );
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(self)
            }
        }
    }
    fn fsmount(self) -> io::Result<MountFd> {
        unsafe {
            let res = bindings::fsmount(
                self.config_fd.as_raw_fd() as c_int,
                self.fsmount_flags,
                self.fsmount_attr_flags,
            );
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(MountFd::from_raw_fd(res as RawFd))
            }
        }
    }
    pub fn build(self) -> io::Result<MountFd> {
        self.fsconfig_create()?.fsmount()
    }
}

pub struct MountFd(OwnedFd);
impl FromRawFd for MountFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}
impl AsFd for MountFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl AsPathFd for MountFd {}
impl MountFd {
    pub fn mount_at(&mut self, to_path: AtPath) -> io::Result<()> {
        let mut move_mount_flags: c_uint = bindings::MOVE_MOUNT_F_EMPTY_PATH;
        let empty_str = c"";
        let to_path_args = to_path.as_syscall_args_ioerr()?;
        if (to_path_args.flags_arg & (bindings::AT_EMPTY_PATH as c_int)) != 0 {
            move_mount_flags |= bindings::MOVE_MOUNT_T_EMPTY_PATH as c_uint;
        }
        unsafe {
            let res = bindings::move_mount(
                self.0.as_raw_fd(),
                empty_str.as_ptr(),
                to_path_args.fd_arg,
                to_path_args.path_arg.as_ptr(),
                move_mount_flags,
            );
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}

pub struct UmountBuilder {
    umount2_flags: c_int,
}
impl UmountBuilder {
    pub fn new() -> Self {
        Self { umount2_flags: 0 }
    }
    pub fn detach(mut self) -> Self {
        self.umount2_flags |= bindings::MNT_DETACH as c_int;
        self
    }
    pub fn exec(self, target: &str) -> io::Result<()> {
        let target_c = CString::new(target)?;
        unsafe {
            let res = bindings::umount2(target_c.as_ptr(), self.umount2_flags);
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}
