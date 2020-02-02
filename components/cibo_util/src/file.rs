// Copyright 2017 TiKV Project Authors. Licensed under Apache-2.0.

use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Read};
use std::path::Path;

use openssl::error::ErrorStack;
use openssl::hash::{self, Hasher, MessageDigest};

pub fn get_file_size<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    let meta = fs::metadata(path)?;
    Ok(meta.len())
}

pub fn path_exists<P: AsRef<Path>>(file: P) -> bool {
    let path = file.as_ref();
    path.exists()
}

pub fn file_exists<P: AsRef<Path>>(file: P) -> bool {
    let path = file.as_ref();
    path.exists() && path.is_file()
}

/// Deletes given path from file system. Returns `true` on success, `false` if the file doesn't exist.
/// Otherwise the raw error will be returned.
pub fn delete_file_if_exist<P: AsRef<Path>>(file: P) -> io::Result<bool> {
    match fs::remove_file(&file) {
        Ok(_) => Ok(true),
        Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// Deletes given path from file system. Returns `true` on success, `false` if the directory doesn't
/// exist. Otherwise the raw error will be returned.
pub fn delete_dir_if_exist<P: AsRef<Path>>(dir: P) -> io::Result<bool> {
    match fs::remove_dir_all(&dir) {
        Ok(_) => Ok(true),
        Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// Creates a new, empty directory at the provided path. Returns `true` on success,
/// `false` if the directory already exists. Otherwise the raw error will be returned.
pub fn create_dir_if_not_exist<P: AsRef<Path>>(dir: P) -> io::Result<bool> {
    match fs::create_dir(&dir) {
        Ok(_) => Ok(true),
        Err(ref e) if e.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

/// Call fsync on directory by its path
pub fn sync_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    // File::open will not error when opening a directory
    // because it just call libc::open and do not do the file or dir check
    fs::File::open(path)?.sync_all()
}

const DIGEST_BUFFER_SIZE: usize = 1024 * 1024;

/// Calculates the given file's Crc32 checksum.
pub fn calc_crc32<P: AsRef<Path>>(path: P) -> io::Result<u32> {
    let mut digest = crc32fast::Hasher::new();
    let mut f = OpenOptions::new().read(true).open(path)?;
    let mut buf = vec![0; DIGEST_BUFFER_SIZE];
    loop {
        match f.read(&mut buf[..]) {
            Ok(0) => {
                return Ok(digest.finalize());
            }
            Ok(n) => {
                digest.update(&buf[..n]);
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
}

/// Calculates the given content's CRC32 checksum.
pub fn calc_crc32_bytes(contents: &[u8]) -> u32 {
    let mut digest = crc32fast::Hasher::new();
    digest.update(contents);
    digest.finalize()
}

pub fn sha256(input: &[u8]) -> Result<Vec<u8>, ErrorStack> {
    hash::hash(MessageDigest::sha256(), input).map(|digest| digest.to_vec())
}

/// Wrapper of a reader which computes its SHA-256 hash while reading.
pub struct Sha256Reader<R> {
    reader: R,
    hasher: Hasher,
}

impl<R> Sha256Reader<R> {
    /// Creates a new `Sha256Reader`, wrapping the given reader.
    pub fn new(reader: R) -> Result<Self, ErrorStack> {
        Ok(Sha256Reader {
            reader,
            hasher: Hasher::new(MessageDigest::sha256())?,
        })
    }

    /// Computes the final SHA-256 hash.
    pub fn hash(mut self) -> Result<Vec<u8>, ErrorStack> {
        Ok(self.hasher.finish()?.to_vec())
    }
}

impl<R: Read> Read for Sha256Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.hasher.update(&buf[..len])?;
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::iter;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_get_file_size() {
        let tmp_dir = TempDir::new().unwrap();
        let dir_path = tmp_dir.path().to_path_buf();

        // Ensure it works to get the size of an empty file.
        let empty_file = dir_path.join("empty_file");
        {
            let _ = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&empty_file)
                .unwrap();
        }
        assert_eq!(get_file_size(&empty_file).unwrap(), 0);

        // Ensure it works to get the size of an non-empty file.
        let non_empty_file = dir_path.join("non_empty_file");
        let size = 5;
        let v = vec![0; size];
        {
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&non_empty_file)
                .unwrap();
            f.write_all(&v[..]).unwrap();
        }
        assert_eq!(get_file_size(&non_empty_file).unwrap(), size as u64);

        // Ensure it works for non-existent file.
        let non_existent_file = dir_path.join("non_existent_file");
        assert!(get_file_size(&non_existent_file).is_err());
    }

    #[test]
    fn test_file_exists() {
        let tmp_dir = TempDir::new().unwrap();
        let dir_path = tmp_dir.path().to_path_buf();

        assert_eq!(file_exists(&dir_path), false);

        let existent_file = dir_path.join("empty_file");
        {
            let _ = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&existent_file)
                .unwrap();
        }
        assert_eq!(file_exists(&existent_file), true);

        let non_existent_file = dir_path.join("non_existent_file");
        assert_eq!(file_exists(&non_existent_file), false);
    }

    #[test]
    fn test_delete_file_if_exist() {
        let tmp_dir = TempDir::new().unwrap();
        let dir_path = tmp_dir.path().to_path_buf();

        let existent_file = dir_path.join("empty_file");
        {
            let _ = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&existent_file)
                .unwrap();
        }
        assert_eq!(file_exists(&existent_file), true);
        delete_file_if_exist(&existent_file).unwrap();
        assert_eq!(file_exists(&existent_file), false);

        let non_existent_file = dir_path.join("non_existent_file");
        delete_file_if_exist(&non_existent_file).unwrap();
    }

    fn gen_rand_file<P: AsRef<Path>>(path: P, size: usize) -> u32 {
        let mut rng = thread_rng();
        let s: String = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(size)
            .collect();
        fs::write(path, s.as_bytes()).unwrap();
        calc_crc32_bytes(s.as_bytes())
    }

    #[test]
    fn test_calc_crc32() {
        let tmp_dir = TempDir::new().unwrap();

        let small_file = tmp_dir.path().join("small.txt");
        let small_checksum = gen_rand_file(&small_file, 1024);
        assert_eq!(calc_crc32(&small_file).unwrap(), small_checksum);

        let large_file = tmp_dir.path().join("large.txt");
        let large_checksum = gen_rand_file(&large_file, DIGEST_BUFFER_SIZE * 4);
        assert_eq!(calc_crc32(&large_file).unwrap(), large_checksum);
    }

    #[test]
    fn test_create_delete_dir() {
        let tmp_dir = TempDir::new().unwrap();
        let subdir = tmp_dir.path().join("subdir");

        assert!(!delete_dir_if_exist(&subdir).unwrap());
        assert!(create_dir_if_not_exist(&subdir).unwrap());
        assert!(!create_dir_if_not_exist(&subdir).unwrap());
        assert!(delete_dir_if_exist(&subdir).unwrap());
    }

    #[test]
    fn test_sync_dir() {
        let tmp_dir = TempDir::new().unwrap();
        sync_dir(tmp_dir.path()).unwrap();
        let non_existent_file = tmp_dir.path().join("non_existent_file");
        sync_dir(non_existent_file).unwrap_err();
    }

    #[test]
    fn test_sha256() {
        let tmp_dir = TempDir::new().unwrap();
        let large_file = tmp_dir.path().join("large.txt");
        gen_rand_file(&large_file, DIGEST_BUFFER_SIZE * 4);

        let large_file_bytes = fs::read(&large_file).unwrap();
        let direct_sha256 = sha256(&large_file_bytes).unwrap();

        let large_file_reader = fs::File::open(&large_file).unwrap();
        let mut sha256_reader = Sha256Reader::new(large_file_reader).unwrap();
        sha256_reader.read_to_end(&mut Vec::new()).unwrap();

        assert_eq!(sha256_reader.hash().unwrap(), direct_sha256);
    }
}
