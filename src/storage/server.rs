use crate::storage::buffer::Buffer;
use crate::storage::buffer::BufferEntry;
use crossbeam::deque::{Worker,Stealer};


pub struct StorageServer {
    write_buffer :Buffer,
}

impl StorageServer {
    pub fn new() -> Self {
        StorageServer {
            write_buffer :Buffer::new(),
        }
    }

    pub fn get_buffer_worker(&mut self) -> Worker<BufferEntry> {
        self.write_buffer.0.clone()
    }

    pub fn get_buffer_stealer(&mut self) -> Stealer<BufferEntry> {
        self.write_buffer.1.clone()
    }
    
}