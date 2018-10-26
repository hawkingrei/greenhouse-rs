use libc;
use rocket::request::Request;
use rocket::response::{self, Responder};
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CacheFile(PathBuf, File);

impl CacheFile {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<CacheFile> {
        let file = File::open(path.as_ref())?;
        Ok(CacheFile(path.as_ref().to_path_buf(), file))
    }

    pub fn decompression(self, result: &mut Vec<u8>) {
        zstd::stream::copy_decode(self.1, result).unwrap();
    }
}

impl<'r> Responder<'r> for CacheFile {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        let mut result = Vec::new();
        self.decompression(&mut result);
        let response = result.respond_to(req)?;
        Ok(response)
    }
}

unsafe fn get_disk_usage<P: AsRef<Path>>(path: P) -> Option<(f64, u64, u64)> {
    let mut buf: libc::statvfs = mem::uninitialized();
    let path = CString::new(path.as_ref().to_str().unwrap().as_bytes()).unwrap();
    libc::statvfs(path.as_ptr(), &mut buf as *mut _);
    let percent_blocks_free = (buf.f_bfree as f64) / (buf.f_blocks as f64) * 100.0;
    let bytes_free = (buf.f_bfree as u64) * (buf.f_bsize as u64);
    let bytes_used = (buf.f_blocks as u64 - buf.f_bfree as u64) * (buf.f_bsize as u64);
    return Some((percent_blocks_free, bytes_free, bytes_used));
}
