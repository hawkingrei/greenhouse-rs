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
use std::thread;
use std::time;

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
        info!("get_all_bloom");
        let mut result: Vec<Record> = Vec::new();
        let blooms = Arc::get_mut(&mut self._all_bloom_fd_).unwrap();
        for bloom in blooms {
            let mut r = Record::new();
            if let Err(e) = r.merge_from_bytes(bloom.as_slice()) {
                info!("fail to get all bloom {:?}", e);
                continue;
            };
            result.push(r);
        }
        return result;
    }

    pub fn append_to_all_bloom(&mut self, r: Record) -> io::Result<()> {
        let blooms = Arc::get_mut(&mut self._all_bloom_fd_).unwrap();
        let context = r.write_to_bytes().unwrap();
        blooms.write(context)
    }
}

#[test]
fn test_bloomgc() {
    let mut gc = new_gc_store(PathBuf::from("./test_data"));
    let mut rec = Record::new();
    let mut now: Timestamp = Timestamp::new();
    now.set_seconds(chrono::Local::now().timestamp());
    {
        rec.set_time(now);
        rec.set_data(vec![1]);
        rec.set_totalPut(1234);
        gc.append_to_all_bloom(rec);
    }

    let ten_millis = time::Duration::from_secs(3);
    thread::sleep(ten_millis);

    let mut rec2 = Record::new();
    now = Timestamp::new();
    now.set_seconds(chrono::Local::now().timestamp());
    {
        rec2.set_time(now);
        rec2.set_data(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        rec2.set_totalPut(12345);
        gc.append_to_all_bloom(rec2);
    }

    let mut rec3 = Record::new();
    now = Timestamp::new();
    now.set_seconds(chrono::Local::now().timestamp());
    {
        rec3.set_time(now);
        rec3.set_data(vec![1, 2, 3]);
        rec3.set_totalPut(123456);
        gc.append_to_all_bloom(rec3);
    }

    let result = gc.get_all_bloom();
    let res1 = result.get(0).unwrap();
    assert_eq!(res1.get_data().to_vec(), vec![1]);
    let res2 = result.get(1).unwrap();
    assert_eq!(
        res2.get_data().to_vec(),
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    );
    let res3 = result.get(2).unwrap();
    assert_eq!(res3.get_data().to_vec(), vec![1, 2, 3]);
}
