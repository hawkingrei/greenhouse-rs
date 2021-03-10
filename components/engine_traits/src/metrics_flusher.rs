// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use std::io;
use std::result::Result;
use std::sync::mpsc::{self, Sender};
use std::thread::{Builder as ThreadBuilder, JoinHandle};
use std::time::{Duration, Instant};

use crate::*;

const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_millis(10_000);
const FLUSHER_RESET_INTERVAL: Duration = Duration::from_millis(60_000);

pub struct MetricsFlusher<A: KvEngine, K: KvEngine, R: KvEngine> {
    pub engines: Engines<A, K, R>,
    interval: Duration,
    handle: Option<JoinHandle<()>>,
    sender: Option<Sender<bool>>,
}

impl<A: KvEngine, K: KvEngine, R: KvEngine> MetricsFlusher<A, K, R> {
    pub fn new(engines: Engines<A, K, R>) -> Self {
        MetricsFlusher {
            engines,
            interval: DEFAULT_FLUSH_INTERVAL,
            handle: None,
            sender: None,
        }
    }

    pub fn set_flush_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        let (kv_db, raft_db, analyzer_db) = (
            self.engines.kv.clone(),
            self.engines.raft.clone(),
            self.engines.analyzer.clone(),
        );
        let interval = self.interval;
        let (tx, rx) = mpsc::channel();
        self.sender = Some(tx);
        let h = ThreadBuilder::new()
            .name("metrics-flusher".to_owned())
            .spawn(move || {
                tikv_alloc::add_thread_memory_accessor();
                let mut last_reset = Instant::now();
                while let Err(mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(interval) {
                    kv_db.flush_metrics("kv");
                    raft_db.flush_metrics("raft");
                    analyzer_db.flush_metrics("analyzer");
                    if last_reset.elapsed() >= FLUSHER_RESET_INTERVAL {
                        kv_db.reset_statistics();
                        raft_db.reset_statistics();
                        analyzer_db.reset_statistics();
                        last_reset = Instant::now();
                    }
                }
                tikv_alloc::remove_thread_memory_accessor();
            })?;

        self.handle = Some(h);
        Ok(())
    }

    pub fn stop(&mut self) {
        let h = self.handle.take();
        if h.is_none() {
            return;
        }
        drop(self.sender.take().unwrap());
        if let Err(e) = h.unwrap().join() {
            error!("join metrics flusher failed"; "err" => ?e);
            return;
        }
    }
}
