use super::spb::Record;
use crate::env::io_posix::PosixAppendFile;
use crate::env::io_posix::PosixOverwriteFile;
use crate::env::EnvOptions;
use crate::env::OverwriteFile;
use protobuf::Message;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub struct gc_store {
    path: PathBuf,
    _today_fd_: Arc<PosixOverwriteFile>,
    _all_bloom_fd_: Arc<PosixAppendFile>,
}

pub fn new_gc_store(p: PathBuf) -> gc_store {
    let mut op: EnvOptions = EnvOptions::default();
    let mut ap: EnvOptions = EnvOptions::default();
    let mut of_path = p.clone();
    of_path.push("today");
    let mut af_path = p.clone();
    af_path.push("all");
    let mut of: PosixOverwriteFile = PosixOverwriteFile::new(of_path, op).unwrap();
    let mut af: PosixAppendFile = PosixAppendFile::new(af_path, ap).unwrap();
    gc_store {
        path: p,
        _today_fd_: Arc::new(of),
        _all_bloom_fd_: Arc::new(af),
    }
}

impl gc_store {
    pub fn save_today_bloom(&mut self, r: Vec<u8>) -> io::Result<()> {
        Arc::get_mut(&mut self._today_fd_).unwrap().write(r)
    }

    pub fn get_today_bloom(self) -> io::Result<Vec<u8>> {
        self._today_fd_.read()
    }

    pub fn get_all_bloom(&mut self) -> Vec<Record> {
        let mut result: Vec<Record> = Vec::new();
        let blooms = Arc::get_mut(&mut self._all_bloom_fd_).unwrap();
        for bloom in blooms {
            let mut r = Record::new();
            r.merge_from_bytes(bloom.as_slice()).unwrap();
            result.push(r);
        }
        return result;
    }

    pub fn append_to_all_bloom(&mut self, r: Record) -> io::Result<()> {
        let blooms = Arc::get_mut(&mut self._all_bloom_fd_).unwrap();
        blooms.write(r.write_to_bytes().unwrap())
    }
}
