//! The upload form's one field, `log_file`, read from a multipart body
//! and checked like Laravel's `required` and `File::default()
//! ->extensions(['txt', 'log'])->max(...)` rules, with their messages.

use axum::extract::Multipart;
use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;

/// An accepted log: its bytes and the name the browser sent, as Symfony
/// cleaned it (the part after the last `/` or `\`).
#[derive(Debug)]
pub struct LogFile {
    pub contents: Vec<u8>,
    pub original_name: String,
}

pub const REQUIRED: &str = "The log file field is required.";
pub const NOT_A_FILE: &str = "The log file field must be a file.";
pub const EXTENSIONS: &str =
    "The log file field must have one of the following extensions: txt, log.";
pub const FAILED: &str = "The log file failed to upload.";

pub fn too_large(max_size_mb: u64) -> String {
    format!(
        "The log file field must not be greater than {} kilobytes.",
        max_size_mb * 1024
    )
}

/// What the field held.
enum Field {
    Missing,
    Text(String),
    File { name: String, contents: Vec<u8> },
}

/// The client name as Symfony's `UploadedFile` keeps it.
pub fn client_name(name: &str) -> String {
    let name = name.replace('\\', "/");
    name.rsplit('/').next().unwrap_or_default().to_owned()
}

/// `strtolower(pathinfo($name, PATHINFO_EXTENSION))`.
pub fn client_extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Reads and checks `log_file`; the error is the field's message.
pub async fn read(mut multipart: Multipart, max_size_mb: u64) -> Result<LogFile, String> {
    let mut field = Field::Missing;
    loop {
        let next = match multipart.next_field().await {
            Ok(Some(next)) => next,
            Ok(None) => break,
            Err(error) => return Err(read_error(&error, max_size_mb)),
        };
        if next.name() != Some("log_file") {
            continue;
        }
        let file_name = next.file_name().map(str::to_owned);
        let bytes = match next.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => return Err(read_error(&error, max_size_mb)),
        };
        // A later occurrence replaces an earlier one, as in PHP.
        field = match file_name {
            // A file input left empty sends a part without a file name.
            Some(name) if name.is_empty() => Field::Missing,
            Some(name) => Field::File {
                name,
                contents: bytes.to_vec(),
            },
            None => Field::Text(String::from_utf8_lossy(&bytes).into_owned()),
        };
    }

    match field {
        Field::Missing => Err(REQUIRED.to_owned()),
        // ConvertEmptyStringsToNull, after TrimStrings.
        Field::Text(text) if crate::filters::str_trim(&text).is_empty() => Err(REQUIRED.to_owned()),
        Field::Text(_) => Err(NOT_A_FILE.to_owned()),
        Field::File { name, contents } => {
            let original_name = client_name(&name);
            if !matches!(client_extension(&original_name).as_str(), "txt" | "log") {
                return Err(EXTENSIONS.to_owned());
            }
            if contents.len() as u64 > max_size_mb * 1024 * 1024 {
                return Err(too_large(max_size_mb));
            }
            Ok(LogFile {
                contents,
                original_name,
            })
        }
    }
}

/// A body over the request limit is a file over the size limit; anything
/// else cut the upload short.
fn read_error(error: &MultipartError, max_size_mb: u64) -> String {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        too_large(max_size_mb)
    } else {
        FAILED.to_owned()
    }
}
