use std::convert::TryInto;

use crate::util::cache_padded::CachePadded;

const RINGBUFFER_SIZE: usize = 2 << 20;

struct RingBuffer {
    _unused1: CachePadded<usize>,
    buffer: [u8; RINGBUFFER_SIZE],
    _unused2: CachePadded<usize>,
    _unused3: CachePadded<usize>,
    write_cursor: CachePadded<usize>,
    reader_cursor: CachePadded<usize>,
    _unused4: CachePadded<usize>,
}

impl Default for RingBuffer {
    fn default() -> Self {
        RingBuffer {
            _unused1: Default::default(),
            buffer: [0; RINGBUFFER_SIZE],
            _unused2: Default::default(),
            _unused3: Default::default(),
            write_cursor: Default::default(),
            reader_cursor: Default::default(),
            _unused4: Default::default(),
        }
    }
}

impl RingBuffer {
    pub fn write(&mut self, index: usize, data: &[u8]) {
        let mut index = index % RINGBUFFER_SIZE;
        if index + data.len() + 8 > RINGBUFFER_SIZE {
            let length = data.len().to_le_bytes();
            unsafe {
                if index + 8 > RINGBUFFER_SIZE {
                    let dst = self.buffer.as_mut_ptr().offset(index as isize);
                    std::ptr::copy_nonoverlapping(length.as_ptr(), dst, RINGBUFFER_SIZE - index);
                    std::ptr::copy_nonoverlapping(
                        length
                            .as_ptr()
                            .offset((RINGBUFFER_SIZE - index).try_into().unwrap()),
                        self.buffer.as_mut_ptr(),
                        index + 8 - RINGBUFFER_SIZE,
                    );

                } else {
                    let dst = self.buffer.as_mut_ptr().offset(index as isize);
                     std::ptr::copy_nonoverlapping(length.as_ptr(), dst, 8);

                }
            }
            index = (index +8) % RINGBUFFER_SIZE;

            let length = usize::to_le_bytes(data.len());
            let (_, right) = self.buffer.split_at_mut(index);
            right.copy_from_slice(&length);
            right.copy_from_slice(&data);
        } else {
            let length = usize::to_le_bytes(data.len());
            let (_, right) = self.buffer.split_at_mut(index);
            right.copy_from_slice(&length);
            right.copy_from_slice(data);
        }
    }

    pub fn read(&mut self, index: usize) -> Vec<u8> {
        let mut index = index % RINGBUFFER_SIZE;
        let mut length = 0;
        if index + 8 > RINGBUFFER_SIZE - 1 {
            let mut length_slice = [0u8; 8];
            let (l_length, r_length) = length_slice.split_at_mut(RINGBUFFER_SIZE - 1 - index);
            l_length.copy_from_slice(&self.buffer[index..RINGBUFFER_SIZE - 1]);
            r_length.copy_from_slice(&self.buffer[0..9 + index - RINGBUFFER_SIZE]);
            index = 9 + index - RINGBUFFER_SIZE;
            length = usize::from_le_bytes(length_slice);
        } else {
            let mut length_slice = [0u8; 8];
            length_slice.copy_from_slice(&self.buffer[index..index + 8]);
            index = index + 8;
            length = usize::from_le_bytes(length_slice);
        };
        if index + length > RINGBUFFER_SIZE - 1 {
            let mut data = Vec::with_capacity(length);
            data.append(&mut self.buffer[index..RINGBUFFER_SIZE - 1].to_vec());
            data.append(&mut self.buffer[0..length + 1 + index - RINGBUFFER_SIZE].to_vec());
            data
        } else {
            self.buffer.get(index..index + length).unwrap().to_vec()
        }
    }
}
