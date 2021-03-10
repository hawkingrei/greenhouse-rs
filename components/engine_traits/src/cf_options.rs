// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use crate::{db_options::TitanDBOptions, sst_partitioner::SstPartitionerFactory};

pub trait ColumnFamilyOptions {
    type TitanDBOptions: TitanDBOptions;

    fn new() -> Self;
    fn get_level_zero_slowdown_writes_trigger(&self) -> u32;
    fn get_level_zero_stop_writes_trigger(&self) -> u32;
    fn get_soft_pending_compaction_bytes_limit(&self) -> u64;
    fn get_hard_pending_compaction_bytes_limit(&self) -> u64;
    fn get_block_cache_capacity(&self) -> u64;
    fn set_block_cache_capacity(&self, capacity: u64) -> Result<(), String>;
    fn set_titandb_options(&mut self, opts: &Self::TitanDBOptions);
    fn get_target_file_size_base(&self) -> u64;
    fn get_disable_auto_compactions(&self) -> bool;
    fn set_sst_partitioner_factory<F: SstPartitionerFactory>(&mut self, factory: F);
}
