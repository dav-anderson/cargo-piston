use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum PistonError {
    CargoParseError(String),

    ReadDirError{ path: PathBuf, source: IoError },

    RemoveSubdirError{ path: PathBuf, source: IoError },

    RemoveFileError{ path: PathBuf, source: IoError },

    Generic(String)
}

impl fmt::Display for PistonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PistonError::CargoParseError(err) => write!(f, "Failed to parse the cargo.toml: {}", err),
            PistonError::ReadDirError { path, .. } => write!(f, "Failed to read directory {:?}", path),
            PistonError::RemoveSubdirError { path, .. } => write!(f, "Failed to remove subdirectory {:?}", path),
            PistonError::RemoveFileError { path, .. } => write!(f, "Failed to remove file {:?}", path),
            PistonError::Generic(err) => write!(f, "Generic Error: {}", err)
        }
    }
}

impl StdError for PistonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PistonError::ReadDirError { source, .. } => Some(source),
            PistonError::RemoveSubdirError { source, .. } => Some(source),
            PistonError::RemoveFileError { source, .. } => Some(source),
            _ => None,
        }
    }
}