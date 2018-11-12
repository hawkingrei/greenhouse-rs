// Copyright 2016 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt;
use std::io::{self, Write};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::path::Path;

use chrono;
use log;
use log::SetLoggerError;
use slog::{self, Drain, Key, OwnedKVList, Record, KV};
use slog_scope::{self, GlobalLoggerGuard};
use slog_stdlog;
use slog_term::{Decorator, RecordDecorator};

pub use slog::Level;

const TIMESTAMP_FORMAT: &str = "%Y/%m/%d %H:%M:%S%.3f";
const ENABLED_TARGETS: &[&str] = &[
    "tikv::",
    "tests::",
    "benches::",
    "integrations::",
    "failpoints::",
    "raft::",
];

pub fn init_log<D>(drain: D, level: Level) -> Result<GlobalLoggerGuard, SetLoggerError>
where
    D: Drain + Send + Sync + 'static + RefUnwindSafe + UnwindSafe,
    <D as slog::Drain>::Err: ::std::fmt::Debug,
{
    let drain = drain.filter_level(level).fuse();

    let logger = slog::Logger::root(drain, slog_o!());

    let guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init_with_level(convert_slog_level_to_log_level(level))?;
    Ok(guard)
}

pub fn init_log_for_tikv_only<D>(
    drain: D,
    level: Level,
) -> Result<GlobalLoggerGuard, SetLoggerError>
where
    D: Drain + Send + Sync + 'static + RefUnwindSafe + UnwindSafe,
    <D as slog::Drain>::Err: ::std::fmt::Debug,
{
    let filtered = drain.filter(|record| {
        ENABLED_TARGETS
            .iter()
            .any(|target| record.module().starts_with(target))
    });
    init_log(filtered, level)
}

pub fn get_level_by_string(lv: &str) -> Option<Level> {
    match &*lv.to_owned().to_lowercase() {
        "critical" => Some(Level::Critical),
        "error" => Some(Level::Error),
        // We support `warn` due to legacy.
        "warning" | "warn" => Some(Level::Warning),
        "debug" => Some(Level::Debug),
        "trace" => Some(Level::Trace),
        "info" => Some(Level::Info),
        _ => None,
    }
}

// The `to_string()` function of `slog::Level` produces values like `erro` and `trce` instead of
// the full words. This produces the full word.
pub fn get_string_by_level(lv: Level) -> &'static str {
    match lv {
        Level::Critical => "critical",
        Level::Error => "error",
        Level::Warning => "warning",
        Level::Debug => "debug",
        Level::Trace => "trace",
        Level::Info => "info",
    }
}

pub fn convert_slog_level_to_log_level(lv: Level) -> log::LogLevel {
    match lv {
        Level::Critical | Level::Error => log::LogLevel::Error,
        Level::Warning => log::LogLevel::Warn,
        Level::Debug => log::LogLevel::Debug,
        Level::Trace => log::LogLevel::Trace,
        Level::Info => log::LogLevel::Info,
    }
}

#[test]
fn test_get_level_by_string() {
    // Ensure UPPER, Capitalized, and lower case all map over.
    assert_eq!(Some(Level::Trace), get_level_by_string("TRACE"));
    assert_eq!(Some(Level::Trace), get_level_by_string("Trace"));
    assert_eq!(Some(Level::Trace), get_level_by_string("trace"));
    // Due to legacy we need to ensure that `warn` maps to `Warning`.
    assert_eq!(Some(Level::Warning), get_level_by_string("warn"));
    assert_eq!(Some(Level::Warning), get_level_by_string("warning"));
    // Ensure that all non-defined values map to `Info`.
    assert_eq!(None, get_level_by_string("Off"));
    assert_eq!(None, get_level_by_string("definitely not an option"));
}

pub struct TikvFormat<D>
where
    D: Decorator,
{
    decorator: D,
}

impl<D> TikvFormat<D>
where
    D: Decorator,
{
    pub fn new(decorator: D) -> Self {
        Self { decorator }
    }
}

impl<D> Drain for TikvFormat<D>
where
    D: Decorator,
{
    type Ok = ();
    type Err = io::Error;

    fn log(&self, record: &Record, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        self.decorator.with_record(record, values, |decorator| {
            let comma_needed = print_msg_header(decorator, record)?;
            {
                let mut serializer = Serializer::new(decorator, comma_needed);

                record.kv().serialize(record, &mut serializer)?;

                values.serialize(record, &mut serializer)?;

                serializer.finish()?;
            }

            decorator.start_whitespace()?;
            writeln!(decorator)?;

            decorator.flush()?;

            Ok(())
        })
    }
}

/// Returns `true` if message was not empty
fn print_msg_header(mut rd: &mut RecordDecorator, record: &Record) -> io::Result<bool> {
    rd.start_timestamp()?;
    write!(rd, "{}", chrono::Local::now().format(TIMESTAMP_FORMAT))?;

    rd.start_whitespace()?;
    write!(rd, " ")?;

    rd.start_level()?;
    write!(rd, "{}", record.level().as_short_str())?;

    rd.start_whitespace()?;
    write!(rd, " ")?;

    rd.start_msg()?; // There is no `start_line`.
    write!(
        rd,
        "{}:{}",
        Path::new(record.file())
            .file_name()
            .and_then(|path| path.to_str())
            .unwrap_or("<error>"),
        record.line()
    )?;

    rd.start_separator()?;
    write!(rd, ":")?;

    rd.start_whitespace()?;
    write!(rd, " ")?;

    rd.start_msg()?;
    let mut count_rd = CountingWriter::new(&mut rd);
    write!(count_rd, "{}", record.msg())?;
    Ok(count_rd.count() != 0)
}

struct CountingWriter<'a> {
    wrapped: &'a mut io::Write,
    count: usize,
}

impl<'a> CountingWriter<'a> {
    fn new(wrapped: &'a mut io::Write) -> CountingWriter {
        CountingWriter { wrapped, count: 0 }
    }

    fn count(&self) -> usize {
        self.count
    }
}

impl<'a> io::Write for CountingWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.wrapped.write(buf).map(|n| {
            self.count += n;
            n
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.wrapped.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.wrapped.write_all(buf).map(|_| {
            self.count += buf.len();
            ()
        })
    }
}

struct Serializer<'a> {
    comma_needed: bool,
    decorator: &'a mut RecordDecorator,
}

impl<'a> Serializer<'a> {
    fn new(decorator: &'a mut RecordDecorator, comma_needed: bool) -> Self {
        Serializer {
            comma_needed,
            decorator,
        }
    }

    fn maybe_print_comma(&mut self) -> io::Result<()> {
        if self.comma_needed {
            self.decorator.start_comma()?;
            write!(self.decorator, ", ")?;
        }
        self.comma_needed |= true;
        Ok(())
    }

    fn finish(self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> Drop for Serializer<'a> {
    fn drop(&mut self) {}
}

macro_rules! s(
    ($s:expr, $k:expr, $v:expr) => {
        $s.maybe_print_comma()?;
        $s.decorator.start_key()?;
        write!($s.decorator, "{}", $k)?;
        $s.decorator.start_separator()?;
        write!($s.decorator, ":")?;
        $s.decorator.start_whitespace()?;
        write!($s.decorator, " ")?;
        $s.decorator.start_value()?;
        write!($s.decorator, "{}", $v)?;
    };
);

#[cfg_attr(feature = "cargo-clippy", allow(write_literal))]
impl<'a> slog::ser::Serializer for Serializer<'a> {
    fn emit_none(&mut self, key: Key) -> slog::Result {
        s!(self, key, "None");
        Ok(())
    }
    fn emit_unit(&mut self, key: Key) -> slog::Result {
        s!(self, key, "()");
        Ok(())
    }

    fn emit_bool(&mut self, key: Key, val: bool) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }

    fn emit_char(&mut self, key: Key, val: char) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }

    fn emit_usize(&mut self, key: Key, val: usize) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_isize(&mut self, key: Key, val: isize) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }

    fn emit_u8(&mut self, key: Key, val: u8) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_i8(&mut self, key: Key, val: i8) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_u16(&mut self, key: Key, val: u16) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_i16(&mut self, key: Key, val: i16) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_u32(&mut self, key: Key, val: u32) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_i32(&mut self, key: Key, val: i32) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_f32(&mut self, key: Key, val: f32) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_u64(&mut self, key: Key, val: u64) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_i64(&mut self, key: Key, val: i64) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_f64(&mut self, key: Key, val: f64) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_str(&mut self, key: Key, val: &str) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
    fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> slog::Result {
        s!(self, key, val);
        Ok(())
    }
}

#[test]
fn test_log_format() {
    use chrono::{TimeZone, Utc};
    use slog::Logger;
    use slog_term::PlainSyncDecorator;
    use std::cell::RefCell;
    use std::io::Write;
    use std::str::from_utf8;

    // Due to the requirements of `Logger::root*` on a writer with a 'static lifetime
    // we need to make a Thread Local,
    // and implement a custom writer.
    thread_local! {
        static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    }
    struct TestWriter;
    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            BUFFER.with(|buffer| buffer.borrow_mut().write(buf))
        }
        fn flush(&mut self) -> io::Result<()> {
            BUFFER.with(|buffer| buffer.borrow_mut().flush())
        }
    }

    // Make the log
    let decorator = PlainSyncDecorator::new(TestWriter);
    let drain = TikvFormat::new(decorator).fuse();
    let logger = Logger::root_typed(drain, slog_o!());
    slog_crit!(logger, "test");

    // Check the logged value.
    BUFFER.with(|buffer| {
        let buffer = buffer.borrow_mut();
        let output = from_utf8(&*buffer).unwrap();

        // This functions roughly as an assert to make sure that the log level and file name is logged.
        let mut split_iter = output.split(" CRIT logger.rs:");
        // The pre-split portion will contain a timestamp which we can check by parsing and ensuring it is valid.
        let datetime = split_iter.next().unwrap();
        assert!(Utc.datetime_from_str(datetime, TIMESTAMP_FORMAT).is_ok());
        // The post-split portion will contain the line number of the file (which we validate is a number), and then the log message.
        let line_and_message = split_iter.next().unwrap();
        let mut split_iter = line_and_message.split(": ");
        // Since the file will change, asserting the number exactly is unmaintainable.
        split_iter
            .next()
            .and_then(|val| val.parse::<usize>().ok())
            .unwrap();
        // We do know the message though!
        let message = split_iter.next().unwrap();
        assert_eq!(message, "test\n");
    });
}
