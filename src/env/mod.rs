pub mod io_posix;

use std::io;
use std::path::PathBuf;

use crate::env;
use crate::util::status::State;

pub const k_default_page_size: usize = 4 * 1024;

#[derive(PartialEq)]
pub enum WALRecoveryMode {
    // Original levelDB recovery
    // We tolerate incomplete record in trailing data on all logs
    // Use case : This is legacy behavior
    KTolerateCorruptedTailRecords = 0x00,
    // Recover from clean shutdown
    // We don't expect to find any corruption in the WAL
    // Use case : This is ideal for unit tests and rare applications that
    // can require high consistency guarantee
    KAbsoluteConsistency = 0x01,
    // Recover to point-in-time consistency (default)
    // We stop the WAL playback on discovering WAL inconsistency
    // Use case : Ideal for systems that have disk controller cache like
    // hard disk, SSD without super capacitor that store related data
    KPointInTimeRecovery = 0x02,
    // Recovery after a disaster
    // We ignore any corruption in the WAL and try to salvage as much data as
    // possible
    // Use case : Ideal for last ditch effort to recover data or systems that
    // operate with low grade unrelated data
    KSkipAnyCorruptedRecords = 0x03,
}

#[derive(Debug, Clone)]
pub struct EnvOptions {
    // If true, then use mmap to read data
    pub use_mmap_reads: bool,

    // If true, then use mmap to write data
    pub use_mmap_writes: bool,

    // If true, then use O_DIRECT for reading data
    pub use_direct_reads: bool,

    // If true, then use O_DIRECT for writing data
    pub use_direct_writes: bool,

    // If false, fallocate() calls are bypassed
    pub allow_fallocate: bool,

    // If true, set the FD_CLOEXEC on open fd.
    pub set_fd_cloexec: bool,

    // If true, we will preallocate the file with FALLOC_FL_KEEP_SIZE flag, which
    // means that file size won't change as part of preallocation.
    // If false, preallocation will also change the file size. This option will
    // improve the performance in workloads where you sync the data on every
    // write. By default, we set it to true for MANIFEST writes and false for
    // WAL writes
    pub fallocate_with_keep_size: bool,

    pub writable_file_max_buffer_size: usize,

    pub bytes_per_sync: usize,
}

impl Default for EnvOptions {
    fn default() -> EnvOptions {
        EnvOptions {
            use_mmap_reads: false,
            use_mmap_writes: true,
            use_direct_reads: false,
            use_direct_writes: true,
            allow_fallocate: true,
            set_fd_cloexec: true,
            fallocate_with_keep_size: true,

            writable_file_max_buffer_size: 1024 * 1024,
            bytes_per_sync: 0,
        }
    }
}

pub trait WritableFile: Sized {
    fn new(filename: String, reopen: bool, preallocation_block_size: usize) -> Self;
    fn append(&mut self, data: Vec<u8>) -> State;
    fn sync(&self) -> State;
    fn close(&self) -> State;
    fn flush(&self) -> State;
    fn fcntl(&self) -> bool;
    fn truncate(&mut self, size: usize) -> State;
    fn get_required_buffer_alignment(&self) -> usize;

    #[cfg(target_os = "linux")]
    fn range_sync(&self, offset: i64, nbytes: i64) -> State;

    #[cfg(not(target_os = "linux"))]
    fn range_sync(&self, offset: i64, nbytes: i64) -> State {
        return State::ok();
    }

    fn allocate(&self, offset: i64, len: i64) -> State {
        return State::ok();
    }

    fn prepare_write(&mut self, offset: usize, len: usize) {}

    fn positioned_append(&mut self, data: Vec<u8>, offset: usize) -> State {
        return State::not_supported();
    }

    fn fsync(&self) -> State {
        return self.sync();
    }

    fn get_file_size(&self) -> usize {
        0
    }

    fn use_direct_io(&self) -> bool {
        false
    }
}

pub trait SequentialFile<RHS = Self>: Sized {
    fn new(filename: String, options: env::EnvOptions, ptr: &mut RHS) -> State;
    fn skip(&self, n: i64) -> State;
    fn read(&mut self, n: usize, mut result: &mut Vec<u8>, scratch: &mut Vec<u8>) -> State;
    fn use_direct_io(&self) -> bool {
        false
    }
}

pub trait OverwriteFile {
    fn read(&self) -> io::Result<Vec<u8>>;
    fn write(&mut self, data: Vec<u8>) -> io::Result<()>;
}
