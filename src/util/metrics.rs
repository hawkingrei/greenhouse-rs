use prometheus::*;
use prometheus::{Counter, Gauge};

lazy_static! {
    pub static ref DiskFree: Gauge = register_gauge!(opts!(
        "bazel_cache_disk_free",
        "Free gb on bazel cache disk"
    ))
    .unwrap();
    pub static ref DiskUsed: Gauge = register_gauge!(opts!(
        "bazel_cache_disk_used",
        "Used gb on bazel cache disk"
    ))
    .unwrap();
    pub static ref DiskTotal: Gauge = register_gauge!(opts!(
        "bazel_cache_disk_total",
        "Total gb on bazel cache disk"
    ))
    .unwrap();
    pub static ref FilesEvicted: Counter = register_counter!(opts!(
        "bazel_cache_evicted_files",
        "number of files evicted since last server start"
    ))
    .unwrap();
    pub static ref ActionCacheHits: Counter = register_counter!(opts!(
        "bazel_cache_cas_hits",
        "Approximate number of Action Cache hits since last server start"
    ))
    .unwrap();
    pub static ref CASHits: Counter = register_counter!(opts!(
        "bazel_cache_action_hits",
        "Approximate number of Content Addressed Storage cache hits since last server start"
    ))
    .unwrap();
    pub static ref ActionCacheMisses: Counter = register_counter!(opts!(
        "bazel_cache_action_misses",
        "Approximate number of Content Addressed Storage cache misses since last server start"
    ))
    .unwrap();
    pub static ref CASMisses: Counter = register_counter!(opts!(
        "bazel_cache_cas_misses",
        "Approximate number of Content Addressed Storage cache misses since last server start"
    ))
    .unwrap();
    pub static ref LastEvictedAccessAge: Gauge = register_gauge!(opts!(
        "bazel_cache_last_evicted_access_age",
        "Hours since last access of most recently evicted file (at eviction time)"
    ))
    .unwrap();
}
