#![feature(ptr_offset_from)]
#![feature(raw_vec_internals)]
#[macro_use]
extern crate alloc;

mod io_uring;
mod result;
mod ringbuffer;
mod serialize;
pub mod util;

use std::io;

pub type LogOffset = u64;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// The system has been used in an unsupported way.
    Unsupported(String),
    /// An unexpected bug has happened. Please open an issue on github!
    ReportableBug(String),
    /// A read or write error has happened when interacting with the file
    /// system.
    Io(io::Error),

    Unresolved,

    // a failpoint has been triggered for testing purposes
    #[doc(hidden)]
    #[cfg(feature = "failpoints")]
    FailPoint,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
