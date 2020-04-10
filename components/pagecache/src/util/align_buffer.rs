use alloc::raw_vec::RawVec;
use std::cmp::min;
use std::fmt::{self, Debug, Formatter};
use std::{ptr, slice};

#[inline]
pub fn truncate_to_page_boundary(page_size: usize, s: usize) -> usize {
    let result = s - (s & (page_size - 1));
    assert!((result % page_size) == 0);
    result
}

#[inline]
fn round_up(x: usize, y: usize) -> usize {
    return ((x + y - 1) / y) * y;
}
#[inline]
fn round_down(x: usize, y: usize) -> usize {
    return (x / y) * y;
}

pub struct AlignedBuffer {
    alignment_: usize,
    buf_: RawVec<u8>,
    capacity_: usize,
    cursize_: usize,
    bufstart_: *mut u8,
}

impl Default for AlignedBuffer {
    fn default() -> Self {
        AlignedBuffer {
            alignment_: 4 * 1024,
            buf_: RawVec::with_capacity(1),
            capacity_: 1,
            cursize_: 0,
            bufstart_: ptr::null_mut::<u8>(),
        }
    }
}

impl AlignedBuffer {
    pub fn get_alignment(&self) -> usize {
        return self.alignment_;
    }

    pub fn get_capacity(&self) -> usize {
        return self.capacity_;
    }

    pub fn get_current_size(&self) -> usize {
        return self.cursize_;
    }

    pub fn alignment(&mut self, alignment: usize) {
        self.alignment_ = alignment;
    }

    pub fn allocate_new_buffer(&mut self, requested_cacacity: usize, copy_data: bool) {
        assert!(self.alignment_ > 0);
        assert!((self.alignment_ & (self.alignment_ - 1)) == 0);
        if copy_data && requested_cacacity < self.cursize_ {
            // If we are downsizing to a capacity that is smaller than the current
            // data in the buffer. Ignore the request.
            return;
        }

        let new_capacity = round_up(requested_cacacity, self.alignment_);
        let new_buf = RawVec::with_capacity_zeroed(new_capacity + 1);
        //let new_bufstart_offset = self.buf_.ptr().align_offset(self.alignment_);
        //let new_bufstart;
        unsafe {
            //new_bufstart = self.buf_.ptr().offset(new_bufstart_offset as isize);
            if copy_data {
                //ptr::write()
                ptr::copy_nonoverlapping(new_buf.ptr(), self.bufstart_, self.cursize_);
            } else {
                self.cursize_ = 0;
            }
        }

        self.bufstart_ = new_buf.ptr();
        self.capacity_ = new_capacity;
        self.buf_ = new_buf;
    }

    pub fn append(&mut self, src: Vec<u8>, append_size: usize) -> usize {
        assert!(self.capacity_ > self.cursize_);
        let buffer_remaining = self.capacity_ - self.cursize_;
        let to_copy = min(append_size, buffer_remaining);
        if to_copy > 0 {
            unsafe {
                ptr::copy_nonoverlapping(
                    src.as_ptr(),
                    self.bufstart_.offset(self.cursize_ as isize),
                    to_copy,
                );
            }
            self.cursize_ += to_copy;
        }
        to_copy
    }

    pub fn read(&mut self, offset: *mut u8, read_size: usize) -> Vec<u8> {
        let mut result = vec![0; read_size];
        let mut to_read = 0;
        unsafe {
            if offset.offset_from(self.buf_.ptr()) < self.cursize_ as isize {
                to_read = min(
                    self.cursize_ - offset.offset_from(self.buf_.ptr()) as usize,
                    read_size,
                );
            }
        }

        if to_read > 0 {
            unsafe {
                ptr::copy_nonoverlapping(offset, result.as_mut_ptr(), to_read);
            }
        }
        //unsafe {
        //    self.bufstart_ = self.bufstart_.offset(read_size as isize);
        //}
        //self.cursize_ = self.cursize_ - read_size;
        result
    }

    pub fn pad_to_aligment_with(&mut self, padding: u8) {
        let total_size = round_up(self.cursize_, self.alignment_);
        let pad_size = total_size - self.cursize_;
        if pad_size > 0 {
            assert!((pad_size + self.cursize_) <= self.capacity_);

            unsafe {
                ptr::write_bytes(
                    self.bufstart_.offset(self.cursize_ as isize),
                    padding,
                    pad_size,
                );
            }
            self.cursize_ += pad_size;
        }
    }

    pub fn pad_with(&mut self, pad_size: usize, padding: u8) {
        assert!((pad_size + self.cursize_) <= self.capacity_);
        unsafe {
            ptr::write_bytes(
                self.bufstart_.offset(self.cursize_ as isize),
                padding,
                pad_size,
            );
        }
        self.cursize_ += pad_size;
    }

    // After a partial flush move the tail to the beginning of the buffer
    pub fn refit_tail(&mut self, tail_offset: usize, tail_size: usize) {
        if tail_size > 0 {
            unsafe {
                ptr::copy(
                    self.bufstart_,
                    self.bufstart_.offset(tail_offset as isize),
                    tail_size,
                );
            }
        }
        self.cursize_ = tail_size;
    }

    pub fn size(&mut self, cursize: usize) {
        self.cursize_ = cursize;
    }

    pub fn buffer_start(&self) -> *mut u8 {
        return self.bufstart_;
    }
}

impl Debug for AlignedBuffer {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "AlignedBuffer[alignment: {}, start: {:?},align start: {:?}, buf: {}]",
            self.alignment_,
            self.buf_.ptr(),
            self.bufstart_,
            escape(unsafe { slice::from_raw_parts(self.buf_.ptr(), self.buf_.capacity()) })
        )
    }
}

pub fn escape(data: &[u8]) -> String {
    let mut escaped = Vec::with_capacity(data.len() * 4);
    for &c in data {
        match c {
            b'\n' => escaped.extend_from_slice(br"\n"),
            b'\r' => escaped.extend_from_slice(br"\r"),
            b'\t' => escaped.extend_from_slice(br"\t"),
            b'"' => escaped.extend_from_slice(b"\\\""),
            b'\\' => escaped.extend_from_slice(br"\\"),
            _ => {
                if c >= 0x20 && c < 0x7f {
                    // c is printable
                    escaped.push(c);
                } else {
                    escaped.push(b'\\');
                    escaped.push(b'0' + (c >> 6));
                    escaped.push(b'0' + ((c >> 3) & 7));
                    escaped.push(b'0' + (c & 7));
                }
            }
        }
    }
    escaped.shrink_to_fit();
    unsafe { String::from_utf8_unchecked(escaped) }
}

#[test]
fn test_aligned_buffer() {
    let mut buf: AlignedBuffer = Default::default();
    buf.alignment(4);
    buf.allocate_new_buffer(16, false);
    let _appended = buf.append(
        String::from("abc").into_bytes(),
        String::from("abc").into_bytes().len(),
    );
    let offset;
    unsafe {
        offset = buf.buffer_start().offset(1);
        let result = buf.read(offset, _appended - 1);
        assert_eq!(result.len(), 2);
        assert_eq!(String::from_utf8_unchecked(result), String::from("bc"));
    }
}

#[test]
fn test_aligned_buffer2() {
    let mut buf: AlignedBuffer = Default::default();
    buf.alignment(4);
    buf.allocate_new_buffer(100, false);
    buf.append(vec![1, 2, 3, 4, 5, 6], vec![1, 2, 3, 4, 5, 6].len());
    let mut offset;

    offset = buf.buffer_start();
    let result = buf.read(offset, 2);
    assert_eq!(result.len(), 2);
    assert_eq!(result, vec![1, 2]);

    unsafe {
        offset = buf.buffer_start();
        let result = buf.read(offset.offset(2), 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result, vec![3, 4, 5]);
    }
}

#[test]
fn test_aligned_buffer3() {
    let mut buf: AlignedBuffer = Default::default();
    buf.alignment(4);
    buf.allocate_new_buffer(100, false);
    let appended = buf.append(vec![1, 2, 3, 4, 5, 6], 6);
    assert_eq!(appended, 6);

    let mut offset;

    offset = buf.buffer_start();
    let result = buf.read(offset, 6);
    assert_eq!(result.len(), 6);
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);

    let appended = buf.append(vec![1, 2, 3, 4, 5, 6, 7], 7);
    assert_eq!(appended, 7);
    unsafe {
        offset = buf.buffer_start();
        let result = buf.read(offset.offset(6), 7);
        assert_eq!(result.len(), 7);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7]);
    }
}
