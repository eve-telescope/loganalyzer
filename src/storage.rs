//! The raw logs: the exact uploaded bytes, gzipped, one file per log at
//! `{LOG_STORAGE_DIR}/{uuid}.txt.gz`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RawLogs {
    dir: PathBuf,
}

/// A stored file that is not a readable gzip stream.
#[derive(Debug)]
pub struct Undecodable;

impl RawLogs {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self, uuid: Uuid) -> PathBuf {
        self.dir.join(format!("{uuid}.txt.gz"))
    }

    pub async fn exists(&self, uuid: Uuid) -> bool {
        tokio::fs::try_exists(self.path(uuid))
            .await
            .unwrap_or(false)
    }

    /// Stores a log, compressed at level 9. The file appears whole or not
    /// at all (written next to it, then renamed).
    pub async fn put(&self, uuid: Uuid, contents: &[u8]) -> std::io::Result<()> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(contents)?;
        let compressed = encoder.finish()?;
        tokio::fs::create_dir_all(&self.dir).await?;
        let path = self.path(uuid);
        let partial = self.dir.join(format!(".{uuid}.txt.gz.partial"));
        tokio::fs::write(&partial, compressed).await?;
        tokio::fs::rename(&partial, &path).await
    }

    pub async fn delete(&self, uuid: Uuid) {
        let _ = tokio::fs::remove_file(self.path(uuid)).await;
    }

    /// The original bytes of a stored log; `None` when there is no file.
    pub async fn get(&self, uuid: Uuid) -> std::io::Result<Option<Result<Vec<u8>, Undecodable>>> {
        match tokio::fs::read(self.path(uuid)).await {
            Ok(compressed) => Ok(Some(gunzip(&compressed))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// The first gzip member's contents, as PHP's `gzdecode` reads them.
pub fn gunzip(compressed: &[u8]) -> Result<Vec<u8>, Undecodable> {
    let mut contents = Vec::new();
    GzDecoder::new(compressed)
        .read_to_end(&mut contents)
        .map_err(|_| Undecodable)?;
    Ok(contents)
}
