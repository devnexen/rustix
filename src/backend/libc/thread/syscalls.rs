//! libc syscalls supporting `rustix::thread`.

use super::super::c;
use super::super::conv::ret;
#[cfg(any(target_os = "android", target_os = "linux"))]
use super::super::conv::{borrowed_fd, ret_c_int};
use super::super::time::types::LibcTimespec;
#[cfg(any(target_os = "android", target_os = "linux"))]
use crate::fd::BorrowedFd;
use crate::io;
#[cfg(any(target_os = "android", target_os = "linux"))]
use crate::process::{Pid, RawNonZeroPid};
#[cfg(not(target_os = "redox"))]
use crate::thread::{NanosleepRelativeResult, Timespec};
use core::mem::MaybeUninit;
#[cfg(not(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "emscripten",
    target_os = "ios",
    target_os = "macos",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "wasi",
)))]
use {crate::thread::ClockId, core::ptr::null_mut};

#[cfg(all(
    any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
    target_env = "gnu",
))]
weak!(fn __clock_nanosleep_time64(c::clockid_t, c::c_int, *const LibcTimespec, *mut LibcTimespec) -> c::c_int);
#[cfg(all(
    any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
    target_env = "gnu",
))]
weak!(fn __nanosleep64(*const LibcTimespec, *mut LibcTimespec) -> c::c_int);

#[cfg(not(any(
    target_os = "dragonfly",
    target_os = "emscripten",
    target_os = "freebsd", // FreeBSD 12 has clock_nanosleep, but libc targets FreeBSD 11.
    target_os = "ios",
    target_os = "macos",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "wasi",
)))]
#[inline]
pub(crate) fn clock_nanosleep_relative(id: ClockId, request: &Timespec) -> NanosleepRelativeResult {
    let mut remain = MaybeUninit::<LibcTimespec>::uninit();
    let flags = 0;

    // 32-bit gnu version: libc has `clock_nanosleep` but it is not y2038 safe by
    // default.
    #[cfg(all(
        any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
        target_env = "gnu",
    ))]
    unsafe {
        if let Some(libc_clock_nanosleep) = __clock_nanosleep_time64.get() {
            match libc_clock_nanosleep(
                id as c::clockid_t,
                flags,
                &request.clone().into(),
                remain.as_mut_ptr(),
            ) {
                0 => NanosleepRelativeResult::Ok,
                err if err == io::Errno::INTR.0 => {
                    NanosleepRelativeResult::Interrupted(remain.assume_init().into())
                }
                err => NanosleepRelativeResult::Err(io::Errno(err)),
            }
        } else {
            clock_nanosleep_relative_old(id, request)
        }
    }

    // Main version: libc is y2038 safe and has `clock_nanosleep`.
    #[cfg(not(all(
        any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
        target_env = "gnu",
    )))]
    unsafe {
        match c::clock_nanosleep(id as c::clockid_t, flags, request, remain.as_mut_ptr()) {
            0 => NanosleepRelativeResult::Ok,
            err if err == io::Errno::INTR.0 => {
                NanosleepRelativeResult::Interrupted(remain.assume_init())
            }
            err => NanosleepRelativeResult::Err(io::Errno(err)),
        }
    }
}

#[cfg(all(
    any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
    target_env = "gnu",
))]
unsafe fn clock_nanosleep_relative_old(id: ClockId, request: &Timespec) -> NanosleepRelativeResult {
    use core::convert::TryInto;
    let tv_sec = match request.tv_sec.try_into() {
        Ok(tv_sec) => tv_sec,
        Err(_) => return NanosleepRelativeResult::Err(io::Errno::OVERFLOW),
    };
    let tv_nsec = match request.tv_nsec.try_into() {
        Ok(tv_nsec) => tv_nsec,
        Err(_) => return NanosleepRelativeResult::Err(io::Errno::INVAL),
    };
    let old_request = c::timespec { tv_sec, tv_nsec };
    let mut old_remain = MaybeUninit::<c::timespec>::uninit();
    let flags = 0;

    match c::clock_nanosleep(
        id as c::clockid_t,
        flags,
        &old_request,
        old_remain.as_mut_ptr(),
    ) {
        0 => NanosleepRelativeResult::Ok,
        err if err == io::Errno::INTR.0 => {
            let old_remain = old_remain.assume_init();
            let remain = Timespec {
                tv_sec: old_remain.tv_sec.into(),
                tv_nsec: old_remain.tv_nsec.into(),
            };
            NanosleepRelativeResult::Interrupted(remain)
        }
        err => NanosleepRelativeResult::Err(io::Errno(err)),
    }
}

#[cfg(not(any(
    target_os = "dragonfly",
    target_os = "freebsd", // FreeBSD 12 has clock_nanosleep, but libc targets FreeBSD 11.
    target_os = "emscripten",
    target_os = "ios",
    target_os = "macos",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "wasi",
)))]
#[inline]
pub(crate) fn clock_nanosleep_absolute(id: ClockId, request: &Timespec) -> io::Result<()> {
    let flags = c::TIMER_ABSTIME;

    // 32-bit gnu version: libc has `clock_nanosleep` but it is not y2038 safe by
    // default.
    #[cfg(all(
        any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
        target_env = "gnu",
    ))]
    {
        if let Some(libc_clock_nanosleep) = __clock_nanosleep_time64.get() {
            match unsafe {
                libc_clock_nanosleep(
                    id as c::clockid_t,
                    flags,
                    &request.clone().into(),
                    null_mut(),
                )
            } {
                0 => Ok(()),
                err => Err(io::Errno(err)),
            }
        } else {
            clock_nanosleep_absolute_old(id, request)
        }
    }

    // Main version: libc is y2038 safe and has `clock_nanosleep`.
    #[cfg(not(all(
        any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
        target_env = "gnu",
    )))]
    match unsafe { c::clock_nanosleep(id as c::clockid_t, flags, request, null_mut()) } {
        0 => Ok(()),
        err => Err(io::Errno(err)),
    }
}

#[cfg(all(
    any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
    target_env = "gnu",
))]
fn clock_nanosleep_absolute_old(id: ClockId, request: &Timespec) -> io::Result<()> {
    use core::convert::TryInto;

    let flags = c::TIMER_ABSTIME;

    let old_request = c::timespec {
        tv_sec: request.tv_sec.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        tv_nsec: request.tv_nsec.try_into().map_err(|_| io::Errno::INVAL)?,
    };
    match unsafe { c::clock_nanosleep(id as c::clockid_t, flags, &old_request, null_mut()) } {
        0 => Ok(()),
        err => Err(io::Errno(err)),
    }
}

#[cfg(not(target_os = "redox"))]
#[inline]
pub(crate) fn nanosleep(request: &Timespec) -> NanosleepRelativeResult {
    let mut remain = MaybeUninit::<LibcTimespec>::uninit();

    // 32-bit gnu version: libc has `nanosleep` but it is not y2038 safe by
    // default.
    #[cfg(all(
        any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
        target_env = "gnu",
    ))]
    unsafe {
        if let Some(libc_nanosleep) = __nanosleep64.get() {
            match ret(libc_nanosleep(&request.clone().into(), remain.as_mut_ptr())) {
                Ok(()) => NanosleepRelativeResult::Ok,
                Err(io::Errno::INTR) => {
                    NanosleepRelativeResult::Interrupted(remain.assume_init().into())
                }
                Err(err) => NanosleepRelativeResult::Err(err),
            }
        } else {
            nanosleep_old(request)
        }
    }

    // Main version: libc is y2038 safe and has `nanosleep`.
    #[cfg(not(all(
        any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
        target_env = "gnu",
    )))]
    unsafe {
        match ret(c::nanosleep(request, remain.as_mut_ptr())) {
            Ok(()) => NanosleepRelativeResult::Ok,
            Err(io::Errno::INTR) => NanosleepRelativeResult::Interrupted(remain.assume_init()),
            Err(err) => NanosleepRelativeResult::Err(err),
        }
    }
}

#[cfg(all(
    any(target_arch = "arm", target_arch = "mips", target_arch = "x86"),
    target_env = "gnu",
))]
unsafe fn nanosleep_old(request: &Timespec) -> NanosleepRelativeResult {
    use core::convert::TryInto;
    let tv_sec = match request.tv_sec.try_into() {
        Ok(tv_sec) => tv_sec,
        Err(_) => return NanosleepRelativeResult::Err(io::Errno::OVERFLOW),
    };
    let tv_nsec = match request.tv_nsec.try_into() {
        Ok(tv_nsec) => tv_nsec,
        Err(_) => return NanosleepRelativeResult::Err(io::Errno::INVAL),
    };
    let old_request = c::timespec { tv_sec, tv_nsec };
    let mut old_remain = MaybeUninit::<c::timespec>::uninit();

    match ret(c::nanosleep(&old_request, old_remain.as_mut_ptr())) {
        Ok(()) => NanosleepRelativeResult::Ok,
        Err(io::Errno::INTR) => {
            let old_remain = old_remain.assume_init();
            let remain = Timespec {
                tv_sec: old_remain.tv_sec.into(),
                tv_nsec: old_remain.tv_nsec.into(),
            };
            NanosleepRelativeResult::Interrupted(remain)
        }
        Err(err) => NanosleepRelativeResult::Err(err),
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[inline]
#[must_use]
pub(crate) fn gettid() -> Pid {
    // `gettid` wasn't supported in glibc until 2.30, and musl until 1.2.2,
    // so use `syscall`.
    // <https://sourceware.org/bugzilla/show_bug.cgi?id=6399#c62>
    weak_or_syscall! {
        fn gettid() via SYS_gettid -> c::pid_t
    }

    unsafe {
        let tid = gettid();
        debug_assert_ne!(tid, 0);
        Pid::from_raw_nonzero(RawNonZeroPid::new_unchecked(tid))
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[inline]
pub(crate) fn setns(fd: BorrowedFd, nstype: c::c_int) -> io::Result<c::c_int> {
    unsafe { ret_c_int(c::setns(borrowed_fd(fd), nstype)) }
}
