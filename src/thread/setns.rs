#![allow(unsafe_code)]

use bitflags::bitflags;
use linux_raw_sys::general::{
    CLONE_NEWCGROUP, CLONE_NEWIPC, CLONE_NEWNET, CLONE_NEWNS, CLONE_NEWPID, CLONE_NEWTIME,
    CLONE_NEWUSER, CLONE_NEWUTS,
};

use crate::backend::c::c_int;
use crate::backend::thread::syscalls;
use crate::fd::BorrowedFd;
use crate::io;

bitflags! {
    /// Thread name space type.
    pub struct ThreadNameSpaceType: u32 {
        /// Time name space.
        const TIME = CLONE_NEWTIME;
        /// Mount name space.
        const MOUNT = CLONE_NEWNS;
        /// Control group (CGroup) name space.
        const CONTROL_GROUP = CLONE_NEWCGROUP;
        /// `Host name` and `NIS domain name` (UTS) name space.
        const HOST_NAME_AND_NIS_DOMAIN_NAME = CLONE_NEWUTS;
        /// Inter-process communication (IPC) name space.
        const INTER_PROCESS_COMMUNICATION = CLONE_NEWIPC;
        /// User name space.
        const USER = CLONE_NEWUSER;
        /// Process ID name space.
        const PROCESS_ID = CLONE_NEWPID;
        /// Network name space.
        const NETWORK = CLONE_NEWNET;
    }
}

/// Type of name space referred to by a link.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum LinkNameSpaceType {
    /// Time name space.
    Time = CLONE_NEWTIME,
    /// Mount name space.
    Mount = CLONE_NEWNS,
    /// Control group (CGroup) name space.
    ControlGroup = CLONE_NEWCGROUP,
    /// `Host name` and `NIS domain name` (UTS) name space.
    HostNameAndNISDomainName = CLONE_NEWUTS,
    /// Inter-process communication (IPC) name space.
    InterProcessCommunication = CLONE_NEWIPC,
    /// User name space.
    User = CLONE_NEWUSER,
    /// Process ID name space.
    ProcessID = CLONE_NEWPID,
    /// Network name space.
    Network = CLONE_NEWNET,
}

/// Reassociate the calling thread with the namespace associated with link referred to by `fd`.
///
/// `fd` must refer to one of the magic links in a `/proc/[pid]/ns/` directory, or a bind mount
/// to such a link.
///
/// # References
/// - [`setns`]
///
/// [`setns`]: https://man7.org/linux/man-pages/man2/setns.2.html
pub fn move_into_link_name_space(
    fd: BorrowedFd,
    allowed_type: Option<LinkNameSpaceType>,
) -> io::Result<()> {
    let allowed_type = allowed_type.map_or(0, |t| t as c_int);
    syscalls::setns(fd, allowed_type).map(|_r| ())
}

/// Atomically move the calling thread into one or more of the same namespaces as the thread
/// referred to by `fd`.
///
/// `fd` must refer to a thread ID. See: `pidfd_open` and `clone`.
///
/// # References
/// - [`setns`]
///
/// [`setns`]: https://man7.org/linux/man-pages/man2/setns.2.html
pub fn move_into_thread_name_spaces(
    fd: BorrowedFd,
    allowed_types: ThreadNameSpaceType,
) -> io::Result<()> {
    syscalls::setns(fd, allowed_types.bits() as c_int).map(|_r| ())
}
