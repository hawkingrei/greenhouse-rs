use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;

use futures::channel::mpsc::UnboundedSender;
use prometheus::IntGauge;
use tokio::runtime::Handle;

use super::metrics::*;

pub struct Stopped<T>(pub T);

impl<T> Display for Stopped<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "channel has been closed")
    }
}

impl<T> Debug for Stopped<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "channel has been closed")
    }
}

impl<T> From<Stopped<T>> for Box<dyn Error + Sync + Send + 'static> {
    fn from(_: Stopped<T>) -> Box<dyn Error + Sync + Send + 'static> {
        box_err!("channel has been closed")
    }
}

pub trait Runnable<T: Display> {
    fn run(&mut self, t: T, handle: &Handle);
    fn shutdown(&mut self) {}
}

/// Scheduler provides interface to schedule task to underlying workers.
pub struct Scheduler<T> {
    name: Arc<String>,
    sender: UnboundedSender<Option<T>>,
    metrics_pending_task_count: IntGauge,
}

impl<T: Display> Scheduler<T> {
    #[allow(dead_code)]
    fn new<S: Into<String>>(name: S, sender: UnboundedSender<Option<T>>) -> Scheduler<T> {
        let name = name.into();
        Scheduler {
            metrics_pending_task_count: WORKER_PENDING_TASK_VEC.with_label_values(&[&name]),
            name: Arc::new(name),
            sender,
        }
    }

    /// Schedules a task to run.
    ///
    /// If the worker is stopped, an error will return.
    pub fn schedule(&self, task: T) -> Result<(), Stopped<T>> {
        debug!("scheduling task {}", task);
        if let Err(err) = self.sender.unbounded_send(Some(task)) {
            return Err(Stopped(err.into_inner().unwrap()));
        }
        self.metrics_pending_task_count.inc();
        Ok(())
    }
}

impl<T: Display> Clone for Scheduler<T> {
    fn clone(&self) -> Scheduler<T> {
        Scheduler {
            name: Arc::clone(&self.name),
            sender: self.sender.clone(),
            metrics_pending_task_count: self.metrics_pending_task_count.clone(),
        }
    }
}
