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

    pub fn decompression(self) -> Box<Vec<u8>> {
        let mut result = Vec::new();
        {
            zstd::stream::copy_decode(self.1, result).unwrap();
        }
        Box::new(result)
    }
}
