#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub struct Bloomfilter {}

impl Bloomfilter {
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
}
