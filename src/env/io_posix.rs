use crate::env;
use crate::env::k_default_page_size;
use crate::env::OverwriteFile;
use crate::env::{SequentialFile, WritableFile};
use crate::util::status::{Code, State};

use libc;
use libc::c_int;
use log::{error, warn};
use std::ffi::CString;
use std::io;
use std::io::ErrorKind;
use std::os::raw::c_char;
use std::path::Path;
use std::path::PathBuf;
use std::usize;

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
    fn init(&mut self, filename: PathBuf, options: env::EnvOptions) -> io::Result<()> {
        let mut fd = -1;
        let mut flag = libc::O_RDWR | libc::O_CREAT;
        let mut file = 0 as *mut libc::FILE;
        loop {
            unsafe {
                fd = libc::open(
                    CString::from_vec_unchecked(
                        filename.as_path().to_str().unwrap().as_bytes().to_vec(),
                    )
                    .as_ptr(),
                    flag,
                    0o644,
                );
                if !(fd < 0 && *errno_location() as i32 == libc::EINTR) {
                    error!("fail to open file");
                    break;
                }
                warn!("{} {} {}", "wait for open", fd, *errno_location());
            }
        }
        if fd < 0 {
            return Err(io::Error::new(
                ErrorKind::Interrupted,
                format!(
                    "While opening a file for sequentially reading: {:?}",
                    filename
                ),
            ));
        }
        SetFD_CLOEXEC(fd, options.clone());
        if options.use_direct_reads && !options.use_mmap_reads {
            #[cfg(target_os = "macos")]
            unsafe {
                if libc::fcntl(fd, libc::F_NOCACHE, 1) == -1 {
                    libc::close(fd);
                    return Err(io::Error::new(
                        ErrorKind::Interrupted,
                        format!("While fcntl NoCache: {:?}", filename),
                    ));
                    //IOError("While fcntl NoCache", fname, errno);
                }
            }
        } else {
            unsafe {
                loop {
                    file = libc::fdopen(fd, b"r".as_ptr() as *const c_char);
                    if !(file == 0 as *mut libc::FILE && *errno_location() as i32 == libc::EINTR) {
                        break;
                    }
                }
                if file == 0 as *mut libc::FILE {
                    libc::close(fd);
                    return Err(io::Error::new(
                        ErrorKind::Interrupted,
                        format!("While opening a file for sequentially read: {:?}", filename),
                    ));
                }
            }
        }
        self.filename_ = filename;
        self.fd_ = fd;
        self.file_ = file;
        return Ok(());
    }
    fn read(&mut self) -> io::Result<Vec<u8>> {
        unsafe {
            let size = libc::lseek(self.file_ as libc::c_int, 0, libc::SEEK_END);
            let mut r = 0;
            let mut scratch: Vec<u8> = vec![0; size as usize];
            let mut result: Vec<u8> = Vec::new();

            r = posix_fread_unlocked(
                scratch.as_mut_ptr() as *mut libc::c_void,
                1 as libc::size_t,
                size as libc::size_t,
                self.file_,
            );

            if (libc::ferror(self.file_) > 0
                && ((*errno_location()) as i32 == libc::EINTR)
                && r == 0)
            {
                return Err(io::Error::new(
                    ErrorKind::Interrupted,
                    format!("While reading file sequentially: {:?}", self.filename_),
                ));
            }
            result.extend_from_slice(scratch.as_slice());
            result.split_off(r);
            if r < size as usize {
                if libc::feof(self.file_) == 0 {
                    return Err(io::Error::new(
                        ErrorKind::Interrupted,
                        format!("While reading file sequentially: {:?}", self.filename_),
                    ));
                } else {
                    clearerr(self.file_);
                }
            }
            return Ok(result);
        }
    }
    fn write(&mut self, data: Vec<u8>) -> io::Result<()> {
        return Ok(());
    }
}
