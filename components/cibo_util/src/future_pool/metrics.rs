use prometheus::*;

lazy_static! {
    pub static ref FUTUREPOOL_RUNNING_TASK_VEC: IntGaugeVec = register_int_gauge_vec!(
        "futurepool_pending_task_total",
        "Current future_pool pending + running tasks.",
        &["name"]
    )
    .unwrap();
    pub static ref FUTUREPOOL_HANDLED_TASK_VEC: IntCounterVec = register_int_counter_vec!(
        "futurepool_handled_task_total",
        "Total number of future_pool handled tasks.",
        &["name"]
    )
    .unwrap();
}
