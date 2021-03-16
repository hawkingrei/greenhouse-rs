use core::arch::x86_64::_mm_pause;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::WRITE_FILE_BUFFER;

#[derive(Clone)]
pub struct WriteFile {
    ctx: Vec<u8>,
    path: PathBuf,
}

impl WriteFile {
    pub fn new(ctx: Vec<u8>, path: PathBuf) -> Self {
        Self { ctx, path }
    }

    pub fn context(self) -> Vec<u8> {
        return self.ctx;
    }

    pub fn path(&self) -> &PathBuf {
        return &self.path;
    }
}

pub struct WriteFileTask {
    basic_path: PathBuf,
}

impl WriteFileTask {
    pub fn new(basic_path: PathBuf) -> WriteFileTask {
        WriteFileTask { basic_path }
    }
    pub fn deal_write_file(&self) -> io::Result<()> {
        while let Ok(data) = WRITE_FILE_BUFFER.pop() {
            let p = self.basic_path.join(data.path());
            let p_parent = p.as_path().parent().unwrap();
            if fs::metadata(p_parent).is_err() {
                fs::create_dir_all(p_parent)?;
            }
            let mut raw_compressed_data: Vec<u8> = vec![];
            zstd::stream::copy_encode(&mut &*data.context(), &mut raw_compressed_data, 3).unwrap();
            let mut file = File::create(p)?;
            file.write_all(&raw_compressed_data)?;
            unsafe {
                _mm_pause();
            }
        }
        return Ok(());
    }
}
