use std::path::PathBuf;

#[derive(Debug)]
pub struct gc_store {
    path: PathBuf,
    _today_fd_: i32,
    _all_fd: i32,
}

pub fn new_gc_store(p: PathBuf) -> gc_store {
    gc_store {
        path: p,
        _today_fd_: 0,
        _all_fd: 0,
    }
}
