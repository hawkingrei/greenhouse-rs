use prometheus::*;
use prometheus::{Counter, Gauge};

lazy_static! {
    pub static ref FILES_EVICTED: Counter = register_counter!(opts!(
        "bazel_cache_evicted_files",
        "number of files evicted since last server start"
    ))
    .unwrap();
    pub static ref ACTION_CACHE_HITS: Counter = register_counter!(opts!(
        "bazel_cache_cas_hits",
        "Approximate number of Action Cache hits since last server start"
    ))
    .unwrap();
    pub static ref CAS_HITS: Counter = register_counter!(opts!(
        "bazel_cache_action_hits",
        "Approximate number of Content Addressed Storage cache hits since last server start"
    ))
    .unwrap();
    pub static ref ACTION_CACHE_MISSES: Counter = register_counter!(opts!(
        "bazel_cache_action_misses",
        "Approximate number of Content Addressed Storage cache misses since last server start"
    ))
    .unwrap();
    pub static ref CAS_MISSES: Counter = register_counter!(opts!(
        "bazel_cache_cas_misses",
        "Approximate number of Content Addressed Storage cache misses since last server start"
    ))
    .unwrap();
    pub static ref LAST_EVICTED_ACCESS_AGE: Gauge = register_gauge!(opts!(
        "bazel_cache_last_evicted_access_age",
        "Hours since last access of most recently evicted file (at eviction time)"
    ))
    .unwrap();
}
