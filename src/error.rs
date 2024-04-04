use std::{fmt::Display, io::ErrorKind, path::PathBuf};

/// An error type that can hold a `PathBuf`` as an optional context to an `io::Error`.
#[derive(Debug)]
pub struct Error {
    path: Option<PathBuf>,
    inner: std::io::Error,
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        self.inner.kind() == ErrorKind::NotFound && self.path.is_some()
    }
}

impl<E: Into<std::io::Error>> From<E> for Error {
    fn from(value: E) -> Self {
        Self {
            path: None,
            inner: value.into(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_not_found() {
            let path = self.path.as_ref().unwrap().display();
            write!(f, "file not found: {path}")
        } else {
            self.inner.fmt(f)
        }
    }
}

pub trait ResultExt<T> {
    fn map_err_path_context(self, path: impl Into<PathBuf>) -> Result<T, Error>;
}

impl<T, E: Into<std::io::Error>> ResultExt<T> for std::result::Result<T, E> {
    fn map_err_path_context(self, path: impl Into<PathBuf>) -> Result<T, Error> {
        self.map_err(|e| Error {
            path: Some(path.into()),
            inner: e.into(),
        })
    }
}
