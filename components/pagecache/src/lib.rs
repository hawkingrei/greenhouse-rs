mod io_uring;
mod result;
mod ringbuffer;
pub mod util;

pub type LogOffset = u64;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
