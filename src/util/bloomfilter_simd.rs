use super::hash_util::rehash32to32;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::cmp;

/// The size of an L1 cache line in bytes on x86-64.    
const CACHE_LINE_SIZE: u8 = 64;
/// log2(number of bytes in a bucket)
const LOG_BUCKET_BYTE_SIZE: u8 = 5;

struct Bucket {
    /// Whether this bucket contains a vaild entry, or it is empty.
    filled: bool,

    /// Used for full outer and right {outer, anti, semi} joins. Indicates whether the
    /// row in the bucket has been matched.
    /// From an abstraction point of view, this is an awkward place to store this
    /// information but it is efficient. This space is otherwise unused.
    matched: bool,

    /// Used in case of duplicates. If true, then the bucketData union should be used as
    /// 'duplicates'.
    hasDuplicates: bool,

    /// Cache of the hash for data.
    /// TODO: Do we even have to cache the hash value?
    hash: u32,
    // Either the data for this bucket or the linked list of duplicates.
    //union {
    //  HtData htdata;
    //  DuplicateNode* duplicates;
    //} bucketData;
}

pub struct Bloomfilter {
    /// log_num_buckets_ is the log (base 2) of the number of buckets in the directory.
    log_num_buckets_: u8,
    /// directory_mask_ is (1 << log_num_buckets_) - 1. It is precomputed for
    /// efficiency reasons.
    directory_mask_: u32,
}

impl Bloomfilter {
    fn new(log_bufferpool_space: u8) -> Bloomfilter {
        let _log_num_buckets = cmp::max(1, log_bufferpool_space - LOG_BUCKET_BYTE_SIZE);
        Bloomfilter {
            log_num_buckets_: _log_num_buckets,
            directory_mask_: (1 << cmp::min(63, _log_num_buckets)) - 1,
        }
    }

    fn directory_size(self) -> u64 {
        return 1 << (self.log_num_buckets_ + LOG_BUCKET_BYTE_SIZE);
    }

    fn make_mask(self, hash: u32) -> __m256i {
        unsafe {
            let ones = _mm256_set1_epi32(1);
            let rehash = _mm256_setr_epi32(
                0x47b6137b as u32 as i32,
                0x44974d91 as u32 as i32,
                0x8824ad5b as u32 as i32,
                0xa2b7289d as u32 as i32,
                0x705495c7 as u32 as i32,
                0x2df1424b as u32 as i32,
                0x9efc4947 as u32 as i32,
                0x5c6bfb31 as u32 as i32,
            );
            let mut hash_data = _mm256_set1_epi32(hash as i32);
            hash_data = _mm256_mullo_epi32(rehash, hash_data);
            hash_data = _mm256_srli_epi32(hash_data, 27);
            return _mm256_sllv_epi32(ones, hash_data);
        }
    }

    fn bucket_insert_avx2(self, bucket_idx: u32, hash: u32) {
        let mask = self.make_mask(hash);
    }

    #[inline(always)]
    #[cfg(target_feature = "avx2")]
    fn insert(self, hash: u32) {
        let bucket_idx = rehash32to32(hash) & self.directory_mask_;
        self.bucket_insert_avx2(bucket_idx, hash)
    }
}
