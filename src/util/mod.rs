pub mod bloomfilter;
//pub mod bloomfilter_simd;
pub mod cache_padded;
pub mod coding;
pub mod hash_util;
#[macro_use]
pub mod macros;
pub mod memory;
pub mod metrics;
pub mod mpsc;
pub mod status;
pub mod test_common;
pub mod time;
pub mod timer;
pub mod worker;

use std::thread;

pub fn get_tag_from_thread_name() -> Option<String> {
    thread::current()
        .name()
        .and_then(|name| name.split("::").skip(1).last())
        .map(From::from)
}
