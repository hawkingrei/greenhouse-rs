use libc;
use prometheus::*;

use std::ffi::CString;
use std::mem;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

lazy_static! {
    pub static ref STORAGE_READ_DURATION_SECONDS_HISTOGRAM_VEC: Histogram = register_histogram!(
        "storage_read_duration_seconds",
        "Bucketed histogram of storage read duration",
        exponential_buckets(0.0005, 2.0, 20).unwrap()
    )
    .unwrap();
    pub static ref STORAGE_WRITE_DURATION_SECONDS_HISTOGRAM_VEC: Histogram = register_histogram!(
        "storage_write_duration_seconds",
        "Bucketed histogram of storage write duration",
        exponential_buckets(0.0005, 2.0, 20).unwrap()
    )
    .unwrap();
    pub static ref DISK_FREE: Gauge = register_gauge!(opts!(
        "bazel_cache_disk_free",
        "Free gb on bazel cache disk"
    ))
    .unwrap();
    pub static ref DISK_USED: Gauge = register_gauge!(opts!(
        "bazel_cache_disk_used",
        "Used gb on bazel cache disk"
    ))
    .unwrap();
    pub static ref DISK_TOTAL: Gauge = register_gauge!(opts!(
        "bazel_cache_disk_total",
        "Total gb on bazel cache disk"
    ))
    .unwrap();
}

pub fn get_disk_usage<P: AsRef<Path>>(path: P) -> Option<(f64, u64, u64)> {
    unsafe {
        let mut buf: libc::statvfs = mem::MaybeUninit::uninit().assume_init();
        let path = CString::new(path.as_ref().to_str().unwrap().as_bytes()).unwrap();
        libc::statvfs(path.as_ptr(), &mut buf as *mut _);
        let percent_blocks_free = (buf.f_bfree as f64) / (buf.f_blocks as f64) * 100.0;
        let bytes_free = (buf.f_bfree as u64) * (buf.f_bsize as u64);
        let bytes_used = (buf.f_blocks as u64 - buf.f_bfree as u64) * (buf.f_bsize as u64);
        Some((percent_blocks_free, bytes_free, bytes_used))
    }
}

pub fn get_disk_usage_prom<P: AsRef<Path>>(path: P) {
    if let Some((_, bytes_free, bytes_used)) = get_disk_usage(path) {
        DISK_FREE.set(bytes_free as f64 / 1.0e9);
        DISK_USED.set(bytes_used as f64 / 1.0e9);
        DISK_TOTAL.set((bytes_free as f64 + bytes_used as f64) / 1.0e9);
    };
}

pub struct DiskMetric {
    metric_handle: Option<thread::JoinHandle<()>>,
    duration: Duration,
    path: PathBuf,
}

impl DiskMetric {
    pub fn new(d: Duration, p: PathBuf) -> DiskMetric {
        DiskMetric {
            metric_handle: None,
            duration: d,
            path: p,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let builder = thread::Builder::new().name("disk-usage-service".to_string());
        let d = self.duration;
        let p = self.path.clone();
        let h = builder.spawn(move || loop {
            info!("disk metric start");
            thread::sleep(d);
            get_disk_usage_prom(p.as_path());
        })?;
        self.metric_handle = Some(h);
        Ok(())
    }
}

impl Drop for DiskMetric {
    fn drop(&mut self) {
        info!("stop disk metric server");
        if let Some(h) = self.metric_handle.take() {
            h.join().unwrap();
        };
    }
}
