use std::mem;
use std::ptr::copy_nonoverlapping;

pub fn encode_fixed32(value: u32) -> [u8; 4] {
    if cfg!(target_endian = "little") {
        unsafe { mem::transmute(value.to_le()) }
    } else {
        unsafe { mem::transmute(value.to_be()) }
    }
}

pub fn encode_fixed64(value: u64) -> [u8; 8] {
    if cfg!(target_endian = "little") {
        unsafe { mem::transmute(value.to_le()) }
    } else {
        unsafe { mem::transmute(value.to_be()) }
    }
}

pub fn decode_fixed32(value: [u8; 4]) -> u32 {
    let mut result: u32 = 0;
    unsafe {
        copy_nonoverlapping(value.as_ptr(), &mut result as *mut u32 as *mut u8, 4);
    }
    if cfg!(target_endian = "little") {
        result.to_le()
    } else {
        result.to_be()
    }
}

pub fn decode_fixed64(value: [u8; 8]) -> u64 {
    let mut result: u64 = 0;
    unsafe {
        copy_nonoverlapping(value.as_ptr(), &mut result as *mut u64 as *mut u8, 8);
    }
    if cfg!(target_endian = "little") {
        result.to_le()
    } else {
        result.to_be()
    }
}
