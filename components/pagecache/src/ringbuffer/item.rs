use crate::serialize::{scoot, Serialize};
use crate::{Error, Result};

use std::convert::TryFrom;
use std::convert::TryInto;
use std::ptr;
use std::intrinsics::transmute;

struct Item {
    length: u64,
    data: Vec<u8>,
}

impl Serialize for Item {
    fn serialized_size(&self) -> u64 {
        let length: u64 = self.length.serialized_size();
        let data_len: u64 = self.data.len().try_into().unwrap();
        return length + data_len;
    }

    fn serialize_into(&self, buf: &mut &mut [u8]) {
        self.length.serialize_into(buf);
        buf.copy_from_slice(&self.data);
        scoot(buf, self.length.try_into().unwrap());
    }

    fn deserialize(buf: &mut &[u8]) -> Result<Self> {
        let length = u64::deserialize(buf)?;
        if buf.len() == (length.serialized_size() + length).try_into().unwrap() {

            unsafe {
                let ret = &buf[..length.try_into().unwrap()];
                let mut data = vec![0; length.try_into().unwrap()];
                ptr::copy_nonoverlapping(data.as_mut_ptr(), *ret.as_ptr() as *mut u8, length.try_into().unwrap());
                return Ok(Item {
                    length: length,
                    data: data,
                });
            }
        }
        return Err(Error::Unresolved);
    }
}
