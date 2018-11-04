use super::hash_util::rehash32to32;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::cmp;
use std::mem;

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

/// A BloomFilter stores sets of items and offers a query operation indicating whether or
/// not that item is in the set.  BloomFilters use much less space than other compact data
/// structures, but they are less accurate: for a small percentage of elements, the query
/// operation incorrectly returns true even when the item is not in the set.
///
/// When talking about Bloom filter size, rather than talking about 'size', which might be
/// ambiguous, we distinguish two different quantities:
///
/// 1. Space: the amount of buffer pool memory used
///
/// 2. NDV: the number of unique items that have been inserted
///
/// BloomFilter is implemented using block Bloom filters from Putze et al.'s "Cache-,
/// Hash- and Space-Efficient Bloom Filters". The basic idea is to hash the item to a tiny
/// Bloom filter the size of a single cache line or smaller. This implementation sets 8
/// bits in each tiny Bloom filter. This provides a false positive rate near optimal for
/// between 5 and 15 bits per distinct value, which corresponds to false positive
/// probabilities between 0.1% (for 15 bits) and 10% (for 5 bits).
///
/// Our tiny BloomFilters are 32 bytes to take advantage of 32-byte SIMD in newer Intel
/// machines. 'noexcept' is added to various functions called from the cross-compiled code
/// so LLVM will not generate exception related code at their call sites.
#[derive(Clone)]
pub struct Bloomfilter {
    /// log_num_buckets_ is the log (base 2) of the number of buckets in the directory.
    log_num_buckets_: u8,
    /// directory_mask_ is (1 << log_num_buckets_) - 1. It is precomputed for
    /// efficiency reasons.
    directory_mask_: u32,

    directory_: Vec<[i64; 4]>,
}

impl Default for Bloomfilter {
    /// Creates an empty `Bloomfilter`.
    fn default() -> Bloomfilter {
        Bloomfilter {
            log_num_buckets_: 0,
            directory_mask_: 0,
            directory_: Vec::new(),
        }
    }
}

impl Bloomfilter {
    pub fn init(self, log_bufferpool_space: u8) -> Bloomfilter {
        let _log_num_buckets = cmp::max(1, log_bufferpool_space - LOG_BUCKET_BYTE_SIZE);
        let alloc_size: usize = self.directory_size();
        let _directory = Vec::with_capacity(alloc_size / mem::size_of::<[i64; 4]>());
        Bloomfilter {
            log_num_buckets_: _log_num_buckets,
            directory_mask_: (1 << cmp::min(63, _log_num_buckets)) - 1,
            directory_: _directory,
        }
    }

    fn directory_size(self) -> usize {
        return 1 << (self.log_num_buckets_ + LOG_BUCKET_BYTE_SIZE);
    }

    fn bucket_insert_avx2(&mut self, bucket_idx: u32, hash: u32) {
        unsafe {
            let mask = make_mask(hash);
            let addr = self.directory_.as_mut_ptr().offset(bucket_idx as isize);
            let mut bucket = _mm256_load_si256(addr as *const __m256i);
            _mm256_store_si256(&mut bucket, _mm256_or_si256(bucket, mask));
            _mm256_zeroupper();
        }
    }

    fn bucket_find_avx2(&self, bucket_idx: u32, hash: u32) -> bool {
        unsafe {
            let mask = make_mask(hash);
            let addr = self.directory_.as_ptr().offset(bucket_idx as isize);
            let bucket = _mm256_load_si256(addr as *const __m256i);
            let result = _mm256_testc_si256(bucket, mask);
            _mm256_zeroupper();
            return result == 0;
        }
    }

    pub fn insert(&mut self, hash: u32) {
        let bucket_idx = rehash32to32(hash) & self.directory_mask_;
        self.bucket_insert_avx2(bucket_idx, hash)
    }

    pub fn find(self, hash: u32) -> bool {
        let bucket_idx = rehash32to32(hash) & self.directory_mask_;
        return self.bucket_find_avx2(bucket_idx, hash);
    }
}

fn make_mask(hash: u32) -> __m256i {
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

#[test]
fn test_bloom_simd() {
    let mut bf: Bloomfilter = Default::default();
    bf = bf.init(8);
    {
        //bf.insert(0x47b2137b);
        //assert!(&bf.find(0x47b2137b) == false);
        assert!(bf.find(0x4) == true);
        //bf.insert(0x48ab59e2);
        //{
        //
        //}
    }
}
