mod bindings;
mod clone;
mod libnl_bindings;
mod linux_syscalls;
mod mount;
mod netlink;

use crate::clone::*;
use crate::linux_syscalls::*;
use crate::mount::*;
use crate::netlink::*;
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::io::Write;

fn set_lo_interface_up() -> Result<(), Box<dyn std::error::Error>> {
    let mut sock = NetlinkSocket::new()?;
    sock.connect(libnl_bindings::NETLINK_ROUTE as i32)?;
    let mut lo_iface =
        get_netlink_route_link_uncached(&mut sock, libnl_bindings::AF_UNSPEC as i32, "lo")?;
    let mut change_req = NetlinkRouteLink::new()?;
    change_req.set_iff_up();
    change_link(&mut sock, &mut lo_iface, &mut change_req, 0)?;
    Ok(())
}

fn remap_to_root_user(uid: UserId, gid: GroupId) -> io::Result<()> {
    let mut uidmap = File::create("/proc/self/uid_map")?;
    let uidmap_contents = format!("0 {} 1", uid.0);
    uidmap.write_all(uidmap_contents.as_bytes())?;
    drop(uidmap);

    let mut setgroups = File::create("/proc/self/setgroups")?;
    setgroups.write_all(b"deny")?;
    drop(setgroups);

    let mut gidmap = File::create("/proc/self/gid_map")?;
    let gidmap_contents = format!("0 {} 1", gid.0);
    gidmap.write_all(gidmap_contents.as_bytes())?;
    drop(gidmap);

    Ok(())
}

fn create_bind_mount_root(image_path: &str) -> io::Result<MountFd> {
    BindMountBuilder::new()
        .recursive()
        .cloexec()
        .attr(
            MountAttrBuilder::new()
                .propagation(MountPropagation::Slave)
                .build(),
        )
        .open(AtPath::Absolute(image_path))
}

fn create_procfs() -> io::Result<MountFd> {
    FilesystemBuilder::new()
        .cloexec()
        .build_config("proc")?
        .cloexec()
        .nodev()
        .nosuid()
        .noexec()
        .build()
}

fn create_sysfs() -> io::Result<MountFd> {
    FilesystemBuilder::new()
        .cloexec()
        .build_config("sysfs")?
        .cloexec()
        .nodev()
        .nosuid()
        .noexec()
        .build()
}

fn create_devpts() -> io::Result<MountFd> {
    FilesystemBuilder::new()
        .cloexec()
        .build_config("devpts")?
        .cloexec()
        .nosuid()
        .noexec()
        .set_string("ptmxmode", "0666")?
        .build()
}

fn create_cgroupfs() -> io::Result<MountFd> {
    FilesystemBuilder::new()
        .cloexec()
        .build_config("cgroup2")?
        .cloexec()
        .nodev()
        .nosuid()
        .noexec()
        .build()
}

fn create_tmpfs(mode: Option<&str>) -> io::Result<MountFd> {
    let mut config = FilesystemBuilder::new()
        .cloexec()
        .build_config("tmpfs")?
        .cloexec()
        .nodev()
        .nosuid();
    if let Some(mode_str) = mode {
        config = config.set_string("mode", mode_str)?;
    }
    config.build()
}

fn setup_root_fs(image_path: &str) -> io::Result<()> {
    let mut root_mount = create_bind_mount_root(image_path)?;
    create_procfs()?.mount_at(root_mount.relpath("proc"))?;
    create_sysfs()?.mount_at(root_mount.relpath("sys"))?;
    create_devpts()?.mount_at(root_mount.relpath("dev/pts"))?;
    create_cgroupfs()?.mount_at(root_mount.relpath("sys/fs/cgroup"))?;
    for dev_to_bind in ["full", "null", "random", "tty", "urandom", "zero"] {
        let dev_path = format!("/dev/{}", dev_to_bind);
        let bind_path = format!("dev/{}", dev_to_bind);
        BindMountBuilder::new()
            .cloexec()
            .open(AtPath::Absolute(dev_path.as_ref()))?
            .mount_at(root_mount.relpath(bind_path.as_ref()))?;
    }
    create_tmpfs(None)?.mount_at(root_mount.relpath("dev/shm"))?;
    create_tmpfs(Some("755"))?.mount_at(root_mount.relpath("run"))?;
    create_tmpfs(None)?.mount_at(root_mount.relpath("tmp"))?;
    root_mount.mount_at(AtPath::Absolute(image_path))?;
    Ok(())
}

fn root_container_chroot(image_path: &str) -> io::Result<()> {
    let new_root_c = CString::new(image_path)?;
    unsafe {
        let res = linux_syscalls::pivot_root(new_root_c.as_ptr(), new_root_c.as_ptr());
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }?;
    chdir("/")?;
    UmountBuilder::new().detach().exec("/")?;
    Ok(())
}

fn root_container_fn(uid: UserId, gid: GroupId) {
    let image_path = "/var/tmp/container-image";
    let res = (|| -> Result<(), Box<dyn std::error::Error>> {
        remap_to_root_user(uid, gid)?;
        set_lo_interface_up()?;
        setup_root_fs(image_path)?;
        root_container_chroot(image_path)?;
        Err(execve(
            "/bin/bash",
            ["-bash"],
            ["PATH=/usr/local/bin:/usr/bin", "USER=root"],
        )
        .into())
    })();
    res.expect("Root container process got error");
}

fn main() -> Result<(), io::Error> {
    set_cloexec(..)?;

    let uid = geteuid();
    let gid = getegid();

    let child_pid = CloneBuilder::new()
        .generate_signal()
        .new_user_namespace()
        .new_pid_namespace()
        .new_mount_namespace()
        .new_cgroup_namespace()
        .new_uts_namespace()
        .new_time_namespace()
        .new_net_namespace()
        .exec(|| root_container_fn(uid, gid))?;

    println!("Root container PID: {}", child_pid.0);

    waitpid(child_pid)?;

    Ok(())
}
