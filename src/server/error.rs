use std::result;

quick_error! {
#[derive(Debug)]
pub enum Error {
}}

pub type Result<T> = result::Result<T, Error>;
