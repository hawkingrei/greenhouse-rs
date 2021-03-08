// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use crate::engine::KvEngine;
use crate::errors::Result;
use crate::options::WriteOptions;
use crate::raft_engine::RaftEngine;

#[derive(Clone, Debug)]
pub struct Engines<A, K, R> {
    pub analyzer: A,
    pub kv: K,
    pub raft: R,
}

impl<A: KvEngine, K: KvEngine, R: RaftEngine> Engines<A, K, R> {
    pub fn new(analyzer_engine: A, kv_engine: K, raft_engine: R) -> Self {
        Engines {
            analyzer: analyzer_engine,
            kv: kv_engine,
            raft: raft_engine,
        }
    }

    pub fn write_analyzer(&self, wb: &A::WriteBatch) -> Result<()> {
        self.analyzer.write(wb)
    }

    pub fn write_analyzer_opt(&self, wb: &A::WriteBatch, opts: &WriteOptions) -> Result<()> {
        self.analyzer.write_opt(wb, opts)
    }

    pub fn sync_analyzer(&self) -> Result<()> {
        self.analyzer.sync()
    }

    pub fn write_kv(&self, wb: &K::WriteBatch) -> Result<()> {
        self.kv.write(wb)
    }

    pub fn write_kv_opt(&self, wb: &K::WriteBatch, opts: &WriteOptions) -> Result<()> {
        self.kv.write_opt(wb, opts)
    }

    pub fn sync_kv(&self) -> Result<()> {
        self.kv.sync()
    }
}
