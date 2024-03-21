use std::{fmt::Display, io::ErrorKind, path::PathBuf};

/// Error type for this program.
#[derive(Debug)]
pub enum Error {
    /// File not found.
    NotFound(PathBuf),
    Other(String),
}
use Error::*;

pub fn from_io_error(path: PathBuf) -> impl Fn(std::io::Error) -> Error {
    move |e: std::io::Error| -> Error {
        match e.kind() {
            ErrorKind::NotFound => NotFound(path.clone()),
            _ => Other(e.to_string()),
        }
    }
}

pub fn from_walkdir_error(path: PathBuf) -> impl Fn(walkdir::Error) -> Error {
    move |e: walkdir::Error| -> Error {
        match e.io_error() {
            Some(_) => NotFound(path.clone()),
            None => Other(e.to_string()),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotFound(path) => write!(f, "file not found: {}", path.display()),
            Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Other(e.to_string())
    }
}
