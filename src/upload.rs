//! The upload form's one field, `log_file`.

/// The client name as Symfony's `UploadedFile` keeps it.
pub fn client_name(_name: &str) -> String {
    unimplemented!("not ported yet")
}

/// `strtolower(pathinfo($name, PATHINFO_EXTENSION))`.
pub fn client_extension(_name: &str) -> String {
    unimplemented!("not ported yet")
}
