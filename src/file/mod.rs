use rocket::request::Request;
use rocket::response::{self, Responder};
use std::fs::File;
use std::io;
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
