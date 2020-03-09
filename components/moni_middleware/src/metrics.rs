use prometheus::*;
lazy_static! {
    pub static ref GREENHOUSE_READING_COUNT: IntCounter = register_int_counter!(opts!(
        "greenhouse_reading_count",
        "greenhouse_reading_count"
    ))
    .unwrap();
    pub static ref GREENHOUSE_WRITING_COUNT: IntCounter = register_int_counter!(opts!(
        "greenhouse_writing_count",
        "greenhouse_writing_count"
    ))
    .unwrap();
    pub static ref TOTAL_TRANSACTION: IntCounter = register_int_counter!(opts!(
        "greenhouse_total_transaction",
        "greenhouse_total_transaction"
    ))
    .unwrap();
    pub static ref GREENHOUSE_BUSINESS_TIMING_SUM: IntCounter = register_int_counter!(opts!(
        "greenhouse_business_timing_sum",
        "greenhouse_business_timing_sum"
    ))
    .unwrap();
    pub static ref GREENHOUSE_BUSINESS_TIMING_COUNT: IntCounter = register_int_counter!(opts!(
        "greenhouse_business_timing_count",
        "greenhouse_business_timing_count"
    ))
    .unwrap();
    pub static ref GREENHOUSE_HTTP_ERROR: Counter = register_counter!(opts!(
        "greenhouse_http_error",
        "Approximate number of http error since last server start"
    ))
    .unwrap();
    pub static ref GREENHOUSE_SIZE_HISTOGRAM: Histogram = register_histogram!(
        "greenhouse_size_histogram",
        "greenhouse_size_histogram",
        exponential_buckets(1024.0, 2.0, 20).unwrap()
    )
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
}
