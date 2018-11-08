pub mod metrics_router;

use crate::config::CachePath;
use crate::disk::CacheFile;
use crate::util::metrics;
use rocket::Data;
use rocket::State;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::path::PathBuf;

#[get("/<file..>")]
pub fn get(file: PathBuf, path: State<CachePath>) -> Option<CacheFile> {
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

            return None;
        }
        Some(result) => {
            if filename.contains("ac") {
                metrics::ActionCacheHits.inc();
            } else {
                metrics::CASHits.inc();
            }
            return Some(result);
        }
    }
}

#[put("/<file..>", data = "<paste>")]
pub fn upload(paste: Data, file: PathBuf, path: State<CachePath>) -> io::Result<String> {
    let filename = match file.to_str() {
        Some(filen) => filen,
        None => return Err(Error::new(ErrorKind::Other, "filename url error")),
    };
    let together = Path::new(&path.0).join(filename.to_string());
    if !together.parent().unwrap().exists() {
        fs::create_dir_all(together.parent().unwrap())?
    }
    let wfile = &mut File::create(&together)?;
    let mut encoder = match zstd::stream::Encoder::new(wfile, 5) {
        Ok(en) => en,
        Err(_) => {
            match fs::remove_file(together.to_str().unwrap().to_string()) {
                Ok(_) => (),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Encoder init error and fail to rm",
                    ));
                }
            }
            return Err(Error::new(ErrorKind::Other, "Encoder init error"));
        }
    };
    match io::copy(&mut paste.open(), &mut encoder) {
        Ok(_) => {
            //empty
        }
        Err(_) => {
            match fs::remove_file(together.to_str().unwrap().to_string()) {
                Ok(_) => (),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Encoder init error and fail to rm",
                    ));
                }
            }
            return Err(Error::new(ErrorKind::Other, "compress init error"));
        }
    };
    encoder.finish().unwrap();
    return Ok(together.to_str().unwrap().to_string());
}

#[head("/<file..>")]
pub fn head(file: PathBuf, path: State<CachePath>) -> Option<()> {
    let filename = match file.to_str() {
        Some(filen) => filen,
        None => return None,
    };
    if Path::new(&path.0).join(filename.to_string()).exists() {
        return Some(());
    }
    return None;
}
