use std::sync::atomic::AtomicUsize;

#[derive(Clone)]
pub struct CachePath(pub String);

lazy_static! {
    pub static ref total_put: AtomicUsize = AtomicUsize::new(0);
}
