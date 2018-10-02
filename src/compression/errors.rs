use snap;
use std::cell;
use std::io;
use std::result;

quick_error! {
  /// Set of errors that can be produced during different operations in Greenhouse.
  #[derive(Debug, PartialEq)]
  pub enum GreenhouseError {
    /// General Greenhouse error.
    /// Returned when code violates normal workflow of working with Greenhouse files.
    General(message: String) {
      display("Greenhouse error: {}", message)
      description(message)
      from(e: io::Error) -> (format!("underlying IO error: {}", e))
      from(e: snap::Error) -> (format!("underlying snap error: {}", e))
      from(e: cell::BorrowMutError) -> (format!("underlying borrow error: {}", e))
    }
    /// "Not yet implemented" Greenhouse error.
    /// Returned when functionality is not yet available.
    NYI(message: String) {
      display("NYI: {}", message)
      description(message)
    }
    /// "End of file" Greenhouse error.
    /// Returned when IO related failures occur, e.g. when there are not enough bytes to
    /// decode.
    EOF(message: String) {
      display("EOF: {}", message)
      description(message)
    }
  }
}

/// A specialized `Result` for Parquet errors.
pub type Result<T> = result::Result<T, GreenhouseError>;

#[macro_export]
macro_rules! general_err {
  ($fmt:expr) => (GreenhouseError::General($fmt.to_owned()));
  ($fmt:expr, $($args:expr),*) => (GreenhouseError::General(format!($fmt, $($args),*)));
  ($e:expr, $fmt:expr) => (GreenhouseError::General($fmt.to_owned(), $e));
  ($e:ident, $fmt:expr, $($args:tt),*) => (
    GreenhouseError::General(&format!($fmt, $($args),*), $e));
}

#[macro_export]
macro_rules! nyi_err {
  ($fmt:expr) => (GreenhouseError::NYI($fmt.to_owned()));
  ($fmt:expr, $($args:expr),*) => (GreenhouseError::NYI(format!($fmt, $($args),*)));
}

#[macro_export]
macro_rules! eof_err {
  ($fmt:expr) => (GreenhouseError::EOF($fmt.to_owned()));
  ($fmt:expr, $($args:expr),*) => (GreenhouseError::EOF(format!($fmt, $($args),*)));
}
