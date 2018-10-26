use crate::file::CacheFile;
use crate::CachePath;
use rocket::Data;
use rocket::State;
use rocket_slog::{SlogFairing, SyncLogger};
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[get("/<file..>")]
pub fn get(file: PathBuf, path: State<CachePath>, logger: SyncLogger) -> Option<CacheFile> {
    //let together = format!("{}/{}", path.0, file.to_str().unwrap().to_string());
    //info!(logger.get(), "formatted: {}", together);
    //println!("{}", together);
    CacheFile::open(Path::new(&path.0).join(file.to_str().unwrap().to_string())).ok()
}

#[put("/<file..>", data = "<paste>")]
pub fn upload(
    paste: Data,
    file: PathBuf,
    path: State<CachePath>,
    logger: SyncLogger,
) -> io::Result<String> {
    let together = Path::new(&path.0).join(file.to_str().unwrap().to_string());
    if !together.parent().unwrap().exists() {
        fs::create_dir_all(together.parent().unwrap())?
    }
    //info!(logger.get(), "formatted: {}", together);
    let wfile = &mut File::create(together)?;
    let mut encoder = zstd::stream::Encoder::new(wfile, 5).unwrap();
    io::copy(&mut paste.open(), &mut encoder).unwrap();
    encoder.finish().unwrap();
    return Ok(file.to_str().unwrap().to_string());
}
