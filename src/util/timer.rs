use crate::util::time::Instant;
use std::cmp::{Ord, Ordering, Reverse};
use std::collections::BinaryHeap;
use std::sync::mpsc;
use std::thread::Builder;
use std::time::Duration;
use tokio_timer::{self, timer::Handle};

pub struct Timer<T> {
    pending: BinaryHeap<Reverse<TimeoutTask<T>>>,
}

impl<T> Timer<T> {
    pub fn new(capacity: usize) -> Self {
        Timer {
            pending: BinaryHeap::with_capacity(capacity),
        }
    }

    /// Add a periodic task into the `Timer`.
    pub fn add_task(&mut self, timeout: Duration, task: T) {
        let task = TimeoutTask {
            next_tick: Instant::now() + timeout,
            task,
        };
        self.pending.push(Reverse(task));
    }

    /// Get the next `timeout` from the timer.
    pub fn next_timeout(&mut self) -> Option<Instant> {
        self.pending.peek().map(|task| task.0.next_tick)
    }

    /// Pop a `TimeoutTask` from the `Timer`, which should be tick before `instant`.
    /// If there is no tasks should be ticked any more, None will be returned.
    ///
    /// The normal use case is keeping `pop_task_before` until get `None` in order
    /// to retreive all avaliable events.
    pub fn pop_task_before(&mut self, instant: Instant) -> Option<T> {
        if self
            .pending
            .peek()
            .map_or(false, |t| t.0.next_tick <= instant)
        {
            return self.pending.pop().map(|t| t.0.task);
        }
        None
    }
}

#[derive(Debug)]
struct TimeoutTask<T> {
    next_tick: Instant,
    task: T,
}

impl<T> PartialEq for TimeoutTask<T> {
    fn eq(&self, other: &TimeoutTask<T>) -> bool {
        self.next_tick == other.next_tick
    }
}

impl<T> Eq for TimeoutTask<T> {}

impl<T> PartialOrd for TimeoutTask<T> {
    fn partial_cmp(&self, other: &TimeoutTask<T>) -> Option<Ordering> {
        self.next_tick.partial_cmp(&other.next_tick)
    }
}

impl<T> Ord for TimeoutTask<T> {
    fn cmp(&self, other: &TimeoutTask<T>) -> Ordering {
        // TimeoutTask.next_tick must have same type of instants.
        self.partial_cmp(other).unwrap()
    }
}

lazy_static! {
    pub static ref GLOBAL_TIMER_HANDLE: Handle = start_global_timer();
}

fn start_global_timer() -> Handle {
    let (tx, rx) = mpsc::channel();
    Builder::new()
        .name(thd_name!("timer"))
        .spawn(move || {
            let mut timer = tokio_timer::Timer::default();
            tx.send(timer.handle()).unwrap();
            loop {
                timer.turn(None).unwrap();
            }
        })
        .unwrap();
    rx.recv().unwrap()
}
