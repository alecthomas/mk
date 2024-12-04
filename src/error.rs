use std::{fmt::Display, io::ErrorKind, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    CommandFailed(i32),
    MissingInput(String),
    MissingOutput(String),
    IO(PathBuf, std::io::Error),
}

impl std::error::Error for Error {}

impl Error {
    pub fn is_not_found(&self) -> bool {
        if let Error::IO(_, e) = self {
            return e.kind() == ErrorKind::NotFound;
        }
        false
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CommandFailed(code) => write!(f, "command failed with code {code}"),
            Error::IO(path, e) => write!(f, "{e} at {path:?}"),
            Error::MissingOutput(path) => {
                write!(f, r#"output "{path}" was not created"#)
            }
            Error::MissingInput(path) => write!(f, r#"input "{path}" does not exist"#),
        }
    }
}

pub trait ResultExt<T> {
    fn map_err_path_context(self, path: impl Into<PathBuf>) -> Result<T, Error>;
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(PathBuf::new(), e)
    }
}

impl<T, E: Into<std::io::Error>> ResultExt<T> for std::result::Result<T, E> {
    fn map_err_path_context(self, path: impl Into<PathBuf>) -> Result<T, Error> {
        self.map_err(|e| Error::IO(path.into(), e.into()))
    }
}
