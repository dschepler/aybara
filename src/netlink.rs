use crate::libnl_bindings;
use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::io;
use std::os::raw::*;
use std::ptr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetlinkError(pub c_int);
impl Display for NetlinkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let c_str = unsafe { CStr::from_ptr(libnl_bindings::nl_geterror(self.0)) };
        let rust_str = c_str.to_string_lossy();
        write!(f, "NetLink error: {}", rust_str)
    }
}
impl std::error::Error for NetlinkError {}

pub struct NetlinkSocket {
    socket: *mut libnl_bindings::nl_sock,
}

impl Drop for NetlinkSocket {
    fn drop(&mut self) {
        unsafe { libnl_bindings::nl_socket_free(self.socket) }
    }
}
impl NetlinkSocket {
    pub fn new() -> io::Result<Self> {
        unsafe {
            let res = libnl_bindings::nl_socket_alloc();
            if res == ptr::null_mut() {
                Err(io::Error::last_os_error())
            } else {
                Ok(NetlinkSocket { socket: res })
            }
        }
    }
    pub fn connect(&mut self, protocol: i32) -> Result<(), NetlinkError> {
        unsafe {
            let res = libnl_bindings::nl_connect(self.socket, protocol);
            if res < 0 {
                Err(NetlinkError(-res))
            } else {
                Ok(())
            }
        }
    }
}

pub struct NetlinkRouteLink {
    link: *mut libnl_bindings::rtnl_link,
}

impl Drop for NetlinkRouteLink {
    fn drop(&mut self) {
        unsafe { libnl_bindings::rtnl_link_put(self.link) }
    }
}
impl NetlinkRouteLink {
    pub fn new() -> io::Result<Self> {
        unsafe {
            let res = libnl_bindings::rtnl_link_alloc();
            if res == ptr::null_mut() {
                Err(io::Error::last_os_error())
            } else {
                Ok(NetlinkRouteLink { link: res })
            }
        }
    }
    pub fn set_flags(&mut self, flags: c_uint) {
        unsafe { libnl_bindings::rtnl_link_set_flags(self.link, flags) }
    }
    pub fn set_iff_up(&mut self) {
        self.set_flags(libnl_bindings::IFF_UP);
    }
}

pub fn get_netlink_route_link_uncached(
    sock: &mut NetlinkSocket,
    addr_family: c_int,
    if_name: &str,
) -> Result<NetlinkRouteLink, NetlinkError> {
    let if_name_c =
        CString::new(if_name).map_err(|_nulerr| NetlinkError(libnl_bindings::NLE_INVAL as i32))?;
    let mut res_ptr: *mut libnl_bindings::rtnl_link = ptr::null_mut();
    unsafe {
        let res = libnl_bindings::rtnl_link_get_kernel(
            sock.socket,
            addr_family,
            if_name_c.as_ptr(),
            &mut res_ptr,
        );
        if res < 0 {
            Err(NetlinkError(-res))
        } else {
            Ok(NetlinkRouteLink { link: res_ptr })
        }
    }
}

pub fn change_link(
    sock: &mut NetlinkSocket,
    link: &mut NetlinkRouteLink,
    change: &mut NetlinkRouteLink,
    flags: c_int,
) -> Result<(), NetlinkError> {
    unsafe {
        let res = libnl_bindings::rtnl_link_change(sock.socket, link.link, change.link, flags);
        if res < 0 {
            Err(NetlinkError(-res))
        } else {
            Ok(())
        }
    }
}
