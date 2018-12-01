pub mod metrics_router;

use crate::config;
use crate::config::CachePath;
use crate::disk::CacheFile;
use crate::util::metrics;
use crossbeam_channel::Sender;
use rocket::Data;
use rocket::State;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tempfile::NamedTempFile;
use zstd::stream::encode_all;

#[get("/<file..>")]
pub fn get(file: PathBuf, path: State<CachePath>, rx: State<Sender<PathBuf>>) -> Option<CacheFile> {
    let filename = match file.to_str() {
        Some(filen) => filen,
        None => return None,
    };
    match CacheFile::open(Path::new(&path.0).join(filename.to_string())).ok() {
        None => {
            if filename.contains("ac") {
                metrics::ActionCacheMisses.inc();
            } else {
                metrics::CASMisses.inc();
            }
            None
        }
        Some(result) => {
            if filename.contains("ac") {
                metrics::ActionCacheHits.inc();
            } else {
                metrics::CASHits.inc();
            }
            rx.send(file).ok();
            Some(result)
        }
    }
}

#[put("/<file..>", data = "<paste>")]
pub fn upload(
    paste: Data,
    file: PathBuf,
    path: State<CachePath>,
    rx: State<Sender<PathBuf>>,
) -> io::Result<String> {
    let filename = match file.to_str() {
        Some(filen) => filen,
        None => return Err(Error::new(ErrorKind::Other, "filename url error")),
    };
    let together = Path::new(&path.0).join(filename.to_string());
    if !together.parent().unwrap().exists() {
        fs::create_dir_all(together.parent().unwrap())?
    }
    let mut wfile = NamedTempFile::new_in(together.parent().unwrap()).unwrap();
    
    let result = match zstd::stream::encode_all(paste.open(),5) {
        Ok(en) => en,
        Err(_) => {
            wfile.close().ok();
            return Err(Error::new(ErrorKind::Other, "Encoder init error"));
        }
    };
   match io::copy(&mut result.as_slice(), &mut wfile){
        Ok(_) => {
            //empty
        }
        Err(_) => {
            wfile.close().ok();
            return Err(Error::new(ErrorKind::Other, "compress init error"));
        }
    };
    /*
    let mut encoder = match zstd::stream::Encoder::new(wfile.as_file_mut(), 5) {
        Ok(en) => en,
        Err(_) => {
            wfile.close().ok();
            return Err(Error::new(ErrorKind::Other, "Encoder init error"));
        }
    };
    match io::copy(&mut paste.open(), &mut encoder) {
        Ok(_) => {
            //empty
        }
        Err(_) => {
            wfile.close().ok();
            return Err(Error::new(ErrorKind::Other, "compress init error"));
        }
    };
    encoder.finish().unwrap();
    */
    fs::rename(wfile.path(), together.clone()).unwrap();
    rx.send(file).ok();
    config::total_put.fetch_add(1, Ordering::SeqCst);
    Ok(together.to_str().unwrap().to_string())
}

#[head("/<file..>")]
pub fn head(file: PathBuf, path: State<CachePath>, rx: State<Sender<PathBuf>>) -> Option<()> {
    let filename = match file.to_str() {
        Some(filen) => filen,
        None => return None,
    };
    if Path::new(&path.0).join(filename.to_string()).exists() {
        rx.send(file).ok();
        return Some(());
    }
    None
}
