use crate::{Error, Result};

use std::convert::{TryFrom, TryInto};

/// Items that may be serialized and deserialized
pub trait Serialize: Sized {
    /// Returns the buffer size required to hold
    /// the serialized bytes for this item.
    fn serialized_size(&self) -> u64;

    /// Serializees the item without allocating.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough.
    fn serialize_into(&self, buf: &mut &mut [u8]);

    /// Attempts to deserialize this type from some bytes.
    fn deserialize(buf: &mut &[u8]) -> Result<Self>;

    /// Returns owned serialized bytes.
    fn serialize(&self) -> Vec<u8> {
        let sz = self.serialized_size();
        let mut buf = vec![0; usize::try_from(sz).unwrap()];
        self.serialize_into(&mut buf.as_mut_slice());
        buf
    }
}

// Moves a reference to mutable bytes forward,
// sidestepping Rust's limitations in reasoning
// about lifetimes.
//
// ☑ Checked with Miri by Tyler on 2019-12-12
#[allow(unsafe_code)]
pub fn scoot(buf: &mut &mut [u8], amount: usize) {
    assert!(buf.len() >= amount);
    let len = buf.len();
    let ptr = buf.as_mut_ptr();
    let new_len = len - amount;

    unsafe {
        let new_ptr = ptr.add(amount);
        *buf = std::slice::from_raw_parts_mut(new_ptr, new_len);
    }
}

impl Serialize for u64 {
    fn serialized_size(&self) -> u64 {
        if *self <= 240 {
            1
        } else if *self <= 2287 {
            2
        } else if *self <= 67823 {
            3
        } else if *self <= 0x00FF_FFFF {
            4
        } else if *self <= 0xFFFF_FFFF {
            5
        } else if *self <= 0x00FF_FFFF_FFFF {
            6
        } else if *self <= 0xFFFF_FFFF_FFFF {
            7
        } else if *self <= 0x00FF_FFFF_FFFF_FFFF {
            8
        } else {
            9
        }
    }

    fn serialize_into(&self, buf: &mut &mut [u8]) {
        let sz = if *self <= 240 {
            buf[0] = u8::try_from(*self).unwrap();
            1
        } else if *self <= 2287 {
            buf[0] = u8::try_from((*self - 240) / 256 + 241).unwrap();
            buf[1] = u8::try_from((*self - 240) % 256).unwrap();
            2
        } else if *self <= 67823 {
            buf[0] = 249;
            buf[1] = u8::try_from((*self - 2288) / 256).unwrap();
            buf[2] = u8::try_from((*self - 2288) % 256).unwrap();
            3
        } else if *self <= 0x00FF_FFFF {
            buf[0] = 250;
            let bytes = self.to_le_bytes();
            buf[1..4].copy_from_slice(&bytes[..3]);
            4
        } else if *self <= 0xFFFF_FFFF {
            buf[0] = 251;
            let bytes = self.to_le_bytes();
            buf[1..5].copy_from_slice(&bytes[..4]);
            5
        } else if *self <= 0x00FF_FFFF_FFFF {
            buf[0] = 252;
            let bytes = self.to_le_bytes();
            buf[1..6].copy_from_slice(&bytes[..5]);
            6
        } else if *self <= 0xFFFF_FFFF_FFFF {
            buf[0] = 253;
            let bytes = self.to_le_bytes();
            buf[1..7].copy_from_slice(&bytes[..6]);
            7
        } else if *self <= 0x00FF_FFFF_FFFF_FFFF {
            buf[0] = 254;
            let bytes = self.to_le_bytes();
            buf[1..8].copy_from_slice(&bytes[..7]);
            8
        } else {
            buf[0] = 255;
            let bytes = self.to_le_bytes();
            buf[1..9].copy_from_slice(&bytes[..8]);
            9
        };

        scoot(buf, sz);
    }

    fn deserialize(buf: &mut &[u8]) -> Result<Self> {
        if buf.is_empty() {
            return Err(Error::Unresolved);
        }
        let (res, scoot) = match buf[0] {
            0..=240 => (u64::from(buf[0]), 1),
            241..=248 => (240 + 256 * (u64::from(buf[0]) - 241) + u64::from(buf[1]), 2),
            249 => (2288 + 256 * u64::from(buf[1]) + u64::from(buf[2]), 3),
            other => {
                let sz = other as usize - 247;
                let mut aligned = [0; 8];
                aligned[..sz].copy_from_slice(&buf[1..=sz]);
                (u64::from_le_bytes(aligned), sz + 1)
            }
        };
        *buf = &buf[scoot..];
        Ok(res)
    }
}

impl Serialize for i64 {
    fn serialized_size(&self) -> u64 {
        8
    }

    fn serialize_into(&self, buf: &mut &mut [u8]) {
        buf[..8].copy_from_slice(&self.to_le_bytes());
        scoot(buf, 8);
    }

    fn deserialize(buf: &mut &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            return Err(Error::Unresolved);
        }

        let array = buf[..8].try_into().unwrap();
        *buf = &buf[8..];
        Ok(i64::from_le_bytes(array))
    }
}

impl Serialize for u32 {
    fn serialized_size(&self) -> u64 {
        4
    }

    fn serialize_into(&self, buf: &mut &mut [u8]) {
        buf[..4].copy_from_slice(&self.to_le_bytes());
        scoot(buf, 4);
    }

    fn deserialize(buf: &mut &[u8]) -> Result<Self> {
        if buf.len() < 4 {
            return Err(Error::Unresolved);
        }

        let array = buf[..4].try_into().unwrap();
        *buf = &buf[4..];
        Ok(u32::from_le_bytes(array))
    }
}

impl Serialize for bool {
    fn serialized_size(&self) -> u64 {
        1
    }

    fn serialize_into(&self, buf: &mut &mut [u8]) {
        let byte = u8::from(*self);
        buf[0] = byte;
        scoot(buf, 1);
    }

    fn deserialize(buf: &mut &[u8]) -> Result<bool> {
        if buf.is_empty() {
            return Err(Error::Unresolved);
        }
        let value = buf[0] != 0;
        *buf = &buf[1..];
        Ok(value)
    }
}

impl Serialize for u8 {
    fn serialized_size(&self) -> u64 {
        1
    }

    fn serialize_into(&self, buf: &mut &mut [u8]) {
        buf[0] = *self;
        scoot(buf, 1);
    }

    fn deserialize(buf: &mut &[u8]) -> Result<u8> {
        if buf.is_empty() {
            return Err(Error::Unresolved);
        }
        let value = buf[0];
        *buf = &buf[1..];
        Ok(value)
    }
}
