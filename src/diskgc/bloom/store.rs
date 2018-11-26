use super::spb::Record;
use crate::env::io_posix::PosixAppendFile;
use crate::env::io_posix::PosixOverwriteFile;
use crate::env::EnvOptions;
use crate::env::OverwriteFile;
use chrono::offset::Local;
use protobuf::well_known_types::Timestamp;
use protobuf::Message;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub struct GcStore {
    path: PathBuf,
    _today_fd_: Arc<PosixOverwriteFile>,
    _all_bloom_fd_: Arc<PosixAppendFile>,
}

pub fn new_gc_store(p: PathBuf) -> GcStore {
    let op: EnvOptions = EnvOptions::default();
    let ap: EnvOptions = EnvOptions::default();
    let mut of_path = p.clone();
    of_path.push("today");
    let mut af_path = p.clone();
    af_path.push("all");
    let of: PosixOverwriteFile = PosixOverwriteFile::new(of_path, op).unwrap();
    let af: PosixAppendFile = PosixAppendFile::new(af_path, ap).unwrap();
    GcStore {
        path: p,
        _today_fd_: Arc::new(of),
        _all_bloom_fd_: Arc::new(af),
    }
}

impl GcStore {
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

#[test]
fn test_bloomgc() {
    let mut gc = new_gc_store(PathBuf::from("./test_data"));
    let mut rec = Record::new();
    let mut now: Timestamp = Timestamp::new();
    now.set_seconds(chrono::Local::now().timestamp());
    rec.set_time(now);
    rec.set_data(vec![1]);
    rec.set_totalPut(1234);
    gc.append_to_all_bloom(rec);
    let ten_millis = time::Duration::from_seconds(3);
    thread::sleep(ten_millis);
    
    now = Timestamp::new();
    now.set_seconds(chrono::Local::now().timestamp());
    rec.set_time(now);
    rec.set_data(vec![1,2]);
    rec.set_totalPut(12345);
    gc.append_to_all_bloom(rec);


    gc.get_all_bloom();
}
