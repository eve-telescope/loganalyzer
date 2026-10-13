//! The raw logs: the exact uploaded bytes, gzipped, one file per log.

/// A stored file that is not a readable gzip stream.
#[derive(Debug)]
pub struct Undecodable;

/// The first gzip member's contents, as PHP's `gzdecode` reads them.
pub fn gunzip(_compressed: &[u8]) -> Result<Vec<u8>, Undecodable> {
    unimplemented!("not ported yet")
}
