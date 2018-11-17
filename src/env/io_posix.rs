use crate::env;
use crate::env::k_default_page_size;
use crate::env::EnvOptions;
use crate::env::OverwriteFile;
use crate::env::{SequentialFile, WritableFile};
use crate::util::status::{Code, State};

use libc;
use libc::c_int;
use log::{error, warn};
use std::ffi::CString;
use std::fmt;
use std::io;
use std::io::ErrorKind;
use std::mem;
use std::os::raw::c_char;
use std::path::Path;
use std::path::PathBuf;
use std::usize;

pub struct FILE(*mut libc::FILE);

unsafe impl Send for FILE {}
unsafe impl Sync for FILE {}

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

pub struct PosixOverwriteFile {
    filename_: PathBuf,
    fd_: i32,
    file_: FILE,
}

impl fmt::Debug for PosixOverwriteFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.filename_)
    }
}

impl PosixOverwriteFile {
    pub fn new(filename: PathBuf, options: env::EnvOptions) -> io::Result<PosixOverwriteFile> {
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
                    file = libc::fdopen(fd, b"rw".as_ptr() as *const c_char);
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

        return Ok(PosixOverwriteFile {
            filename_: filename,
            fd_: fd,
            file_: FILE(file),
        });
    }
}

impl Default for PosixOverwriteFile {
    fn default() -> PosixOverwriteFile {
        PosixOverwriteFile {
            filename_: PathBuf::from(""),
            fd_: 0,
            file_: FILE(0 as *mut libc::FILE),
        }
    }
}

impl Drop for PosixOverwriteFile {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // (opened after we closed ours.
        let _ = unsafe { libc::close(self.fd_) };
    }
}

impl OverwriteFile for PosixOverwriteFile {
    fn read(&mut self) -> io::Result<Vec<u8>> {
        unsafe {
            let mut dst: libc::stat = mem::uninitialized();
            libc::fstat(self.fd_, &mut dst as *mut libc::stat);
            let size = dst.st_size;
            let mut r = 0;
            let mut scratch: Vec<u8> = vec![0; size as usize];

            libc::lseek(self.fd_, 0, libc::SEEK_SET);
            r = posix_fread_unlocked(
                scratch.as_mut_ptr() as *mut libc::c_void,
                size as libc::size_t,
                1 as libc::size_t,
                self.file_.0,
            );

            if (libc::ferror(self.file_.0) > 0
                && ((*errno_location()) as i32 == libc::EINTR)
                && r == 0)
            {
                return Err(io::Error::new(
                    ErrorKind::Interrupted,
                    format!("While reading file sequentially: {:?}", self.filename_),
                ));
            }
            return Ok(scratch);
        }
    }

    fn write(&mut self, data: Vec<u8>) -> io::Result<()> {
        unsafe {
            if libc::ftruncate(self.fd_, 0) < 0 {
                return Err(io::Error::new(ErrorKind::Interrupted, "fail to ftruncate"));
            }
            if libc::lseek(self.fd_, 0, libc::SEEK_SET) < 0 {
                return Err(io::Error::new(ErrorKind::Interrupted, "fail to lseek"));
            }
            if libc::write(self.fd_, data.as_ptr() as *const libc::c_void, data.len()) < 0 {
                return Err(io::Error::new(ErrorKind::Interrupted, "fail to fwrite"));
            }
            return Ok(());
        }
    }
}

#[test]
fn test_overwrite_file() {
    let mut op: EnvOptions = EnvOptions::default();
    let mut of: PosixOverwriteFile = PosixOverwriteFile::new(PathBuf::from("./test"), op).unwrap();
    of.write("abc".as_bytes().to_vec()).unwrap();
    assert_eq!(of.read().unwrap(), "abc".as_bytes().to_vec());
    of.write("qwe".as_bytes().to_vec()).unwrap();
    assert_eq!(of.read().unwrap(), "qwe".as_bytes().to_vec());
}
