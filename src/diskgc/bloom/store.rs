use super::spb::Record;
use crate::env::io_posix::PosixOverwriteFile;
use crate::env::EnvOptions;
use crate::env::OverwriteFile;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub struct gc_store {
    path: PathBuf,
    _today_fd_: Arc<PosixOverwriteFile>,
}

pub fn new_gc_store(p: PathBuf) -> gc_store {
    let mut op: EnvOptions = EnvOptions::default();
    let mut of: PosixOverwriteFile = PosixOverwriteFile::new(p.clone(), op).unwrap();
    gc_store {
        path: p,
        _today_fd_: Arc::new(of),
    }
}

impl gc_store {
    pub fn save_today_bloom(&mut self, r: Vec<u8>) -> io::Result<()> {
        Arc::get_mut(&mut self._today_fd_).unwrap().write(r)
    }
}
