use std::path::Path;

#[derive(Debug)]
pub struct gc_store {
    path: String,
    _today_fd_: i32,
    _all_fd: i32,
}

pub fn new_gc_store(path: &Path) {}
