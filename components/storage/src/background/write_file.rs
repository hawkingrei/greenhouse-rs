use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

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

    pub async fn deal_write_file(&self) -> io::Result<()> {
        while let Some(data) = WRITE_FILE_BUFFER.pop() {
            let p = self.basic_path.join(data.path());
            let p_parent = p.as_path().parent().unwrap();
            if fs::metadata(p_parent).await.is_err() {
                fs::create_dir_all(p_parent).await?;
            }
            let mut raw_compressed_data: Vec<u8> = vec![];
            zstd::stream::copy_encode(&mut &*data.context(), &mut raw_compressed_data, 3).unwrap();
            let mut file = File::create(p).await?;
            file.write_all(&raw_compressed_data).await?;
        }
        return Ok(());
    }
}
