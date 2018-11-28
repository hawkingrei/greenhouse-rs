use crate::util::coding::encode_fixed32;
use std::mem;
use std::str;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Code {
    KOk = 0,
    KNotFound = 1,
    KCorruption = 2,
    KNotSupported = 3,
    KInvalidArgument = 4,
    KIOError = 5,
}

#[derive(Debug, Clone)]
pub struct State {
    state_: Vec<u8>,
}

impl State {
    pub fn new(code: Code, msg1: String, msg2: String) -> State {
        let msg = format!("{}: {}", msg1, msg2);
        let size = mem::size_of_val(&msg);
        let mut state: Vec<u8> = Vec::with_capacity(size + 5);
        state.extend(encode_fixed32(size as u32).iter().cloned());
        state.extend([code as u8].iter().cloned());
        state.append(&mut msg.into_bytes());
        State { state_: state }
    }

    pub fn ok() -> State {
        State::new(Code::KOk, "".to_string(), "".to_string())
    }

    pub fn not_supported() -> State {
        State::new(Code::KNotSupported, "".to_string(), "".to_string())
    }

    pub fn is_ok(&self) -> bool {
        self.state_[4] as u8 == Code::KOk as u8
    }

    pub fn to_string<'a>(s: &'a State) -> &'a str {
        str::from_utf8(&s.state_[5..]).unwrap()
    }
}

#[test]
fn test_state() {
    let s = State::new(Code::KOk, String::from("a"), String::from("b"));
    assert_eq!(true, s.is_ok());
    assert_eq!(&String::from("a: b"), State::to_string(&s))
}
