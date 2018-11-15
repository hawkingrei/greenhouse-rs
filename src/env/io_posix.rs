use crate::env;
use crate::env::k_default_page_size;
use crate::env::{SequentialFile, WritableFile};
use crate::util::status::{Code, State};
use crate::env::OverwriteFile;

use libc;
use libc::c_int;
use std::ffi::CString;
use std::os::raw::c_char;
use std::usize;
use std::path::Path;
use std::path::PathBuf;
use std::io;

pub fn clearerr(stream: *mut libc::FILE) {
    extern "C" {
        fn clearerr(stream: *mut libc::FILE);
    }
    unsafe {
        clearerr(stream);
    }
}

#[cfg(any(target_os = "macos"))]
unsafe fn posix_fread_unlocked(
    ptr: *mut libc::c_void,
    size: libc::size_t,
    nobj: libc::size_t,
    stream: *mut libc::FILE,
) -> libc::size_t {
    return libc::fread(ptr, size, nobj, stream);
}

#[cfg(any(target_os = "linux"))]
unsafe fn posix_fread_unlocked(
    ptr: *mut libc::c_void,
    size: libc::size_t,
    nobj: libc::size_t,
    stream: *mut libc::FILE,
) -> libc::size_t {
    return libc::fread_unlocked(ptr, size, nobj, stream);
}

fn SetFD_CLOEXEC(fd: i32, options: env::EnvOptions) {
    if options.set_fd_cloexec && fd > 0 {
        unsafe {
            libc::fcntl(
                fd,
                libc::F_SETFD,
                libc::fcntl(fd, libc::F_GETFD) | libc::FD_CLOEXEC,
            );
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
unsafe fn errno_location() -> *const c_int {
    extern "C" {
        fn __error() -> *const c_int;
    }
    __error()
}

#[cfg(target_os = "bitrig")]
fn errno_location() -> *const c_int {
    extern "C" {
        fn __errno() -> *const c_int;
    }
    unsafe { __errno() }
}

#[cfg(target_os = "dragonfly")]
unsafe fn errno_location() -> *const c_int {
    extern "C" {
        fn __dfly_error() -> *const c_int;
    }
    __dfly_error()
}

#[cfg(target_os = "openbsd")]
unsafe fn errno_location() -> *const c_int {
    extern "C" {
        fn __errno() -> *const c_int;
    }
    __errno()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn errno_location() -> *const c_int {
    extern "C" {
        fn __errno_location() -> *const c_int;
    }
    __errno_location()
}

#[cfg(target_os = "macos")]
fn get_flag() -> i32 {
    libc::O_CREAT
}

#[cfg(any(
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "linux",
    target_os = "netbsd"
))]
fn get_flag() -> i32 {
    libc::O_CREAT | libc::O_DIRECT
}

fn get_logical_buffer_size() -> usize {
    if cfg!(not(target_os = "linux")) {
        return k_default_page_size;
    } else {
        return k_default_page_size;
        //Todo: support linux
    }
}

fn IsSectorAligned(off: usize, sector_size: usize) -> bool {
    return off % sector_size == 0;
}

#[cfg(target_os = "macos")]
fn get_flag_for_posix_sequential_file() -> i32 {
    0
}

#[cfg(any(
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "linux",
    target_os = "netbsd"
))]
fn get_flag_for_posix_sequential_file() -> i32 {
    libc::O_DIRECT
}

#[derive(Debug)]
pub struct PosixOverwriteFile {
    filename_: PathBuf,
    fd_: i32,
    file_: *mut libc::FILE,
}

impl Default for PosixOverwriteFile {
    fn default() -> PosixOverwriteFile {
        PosixOverwriteFile {
            filename_: PathBuf::from(""),
            fd_: 0,
            file_: 0 as *mut libc::FILE,
        }
    }
}

impl OverwriteFile for PosixOverwriteFile {
    fn init(filename: PathBuf, options: env::EnvOptions) -> io::Result<()> {
        return Ok(());
    }
    fn read(&mut self) -> io::Result<Vec<u8>> {
        return Ok(vec![1, 2]);
    }
    fn write(&mut self, data: Vec<u8>) -> io::Result<()> {
        return Ok(());
    }
}
