pub mod basic;
#[macro_use]
pub mod errors;
#[macro_use]
pub mod compress;

pub use self::basic::Compression as CodecType;
