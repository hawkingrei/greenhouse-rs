#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Compression {
    UNCOMPRESSED,
    SNAPPY,
    GZIP,
    LZO,
    BROTLI,
    LZ4,
    ZSTD,
}
