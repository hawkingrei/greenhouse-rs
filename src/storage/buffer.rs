use crossbeam::deque::{Worker,Stealer};
use crossbeam::deque::lifo;
use crossbeam::deque;
use std::path::PathBuf;

pub struct BufferEntry{
    pub context: Vec<u8>,
    pub address: PathBuf
}

#[derive(Debug)]
pub struct Buffer(pub Worker<BufferEntry>,pub Stealer<BufferEntry>);

impl Buffer{
    pub fn new() -> Buffer {
        let (w, s) = deque::lifo::<BufferEntry>();
        Buffer(w,s)
    }
}