use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;
use std::io::Error as IoError;
use image::ImageError;

#[derive(Debug)]
pub enum PistonError {
    BuildError(String),

    CargoParseError(String),

    WriteFileError(String),

    FileFlushError(String),

    WriteImageError(ImageError),
    
    SaveImageError(String),

    ZigbuildMissingError(String),

    HomebrewMissingError(String),

    ParseManifestError(String),

    CreateManifestError(String),

    WriteManifestError(String),

    AndroidConfigError(String),

    UnsupportedOSError{ os: String, target: String },

    CopyFileError{ input_path: PathBuf, output_path: PathBuf, source: IoError },

    MacOSIconError{ input_path: PathBuf, output_path: PathBuf, source: IoError },

    ReadDirError{ path: PathBuf, source: IoError },

    RemoveSubdirError{ path: PathBuf, source: IoError },

    RemoveFileError{ path: PathBuf, source: IoError },

    CreateFileError{ path: PathBuf, source: IoError },

    CreateDirAllError{ path: PathBuf, source: IoError },

    OpenImageError{ path: PathBuf, source: ImageError },

    Generic(String)
}

impl fmt::Display for PistonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PistonError::BuildError(err) => write!(f, "Failed to build the binary: {}", err),
            PistonError::CargoParseError(err) => write!(f, "Failed to parse the cargo.toml: {}", err),
            PistonError::WriteFileError(err) => write!(f, "Failed to write file: {}", err),
            PistonError::WriteImageError(err) => write!(f, "Failed to write image: {}", err),
            PistonError::SaveImageError(err) => write!(f, "Failed to save image: {}", err),
            PistonError::FileFlushError(err) => write!(f, "Failed to flush file: {}", err),
            PistonError::ZigbuildMissingError(err) => write!(f, "Failed to find zigbuild path in .env file: {}", err),
            PistonError::HomebrewMissingError(err) => write!(f, "Failed to find homebrew bin path in .env file: {}", err),
            PistonError::ParseManifestError(err) => write!(f, "Failed to parse package.metadata.android value: {}", err),
            PistonError::CreateManifestError(err) => write!(f, "Failed to create AndroidManifest.xml file: {}", err),
            PistonError::WriteManifestError(err) => write!(f, "Failed to write AndroidManifest.xml contents: {}", err),
            PistonError::AndroidConfigError(err) => write!(f, "Failed to read android config path from .env: {}", err),
            PistonError::UnsupportedOSError{ os, target, .. } => write!(f, "Host system: {:?} does not support the target: {:?}", os, target),
            PistonError::CopyFileError { input_path, output_path, .. } => write!(f, "Failed to copy {:?} to {:?}", input_path, output_path),
            PistonError::MacOSIconError { input_path, output_path, .. } => write!(f, "Failed to format icon {:?} to {:?}", input_path, output_path),
            PistonError::ReadDirError { path, .. } => write!(f, "Failed to read directory {:?}", path),
            PistonError::RemoveSubdirError { path, .. } => write!(f, "Failed to remove subdirectory {:?}", path),
            PistonError::RemoveFileError { path, .. } => write!(f, "Failed to remove file {:?}", path),
            PistonError::CreateFileError { path, .. } => write!(f, "Failed to Create file {:?}", path),
            PistonError::CreateDirAllError { path, .. } => write!(f, "Failed to create dir all {:?}", path),
            PistonError::OpenImageError { path, .. } => write!(f, "Failed to open image {:?}", path),
            PistonError::Generic(err) => write!(f, "Generic Error: {}", err)
        }
    }
}

impl StdError for PistonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PistonError::CopyFileError { source, .. } => Some(source),
            PistonError::MacOSIconError { source, .. } => Some(source),
            PistonError::ReadDirError { source, .. } => Some(source),
            PistonError::RemoveSubdirError { source, .. } => Some(source),
            PistonError::RemoveFileError { source, .. } => Some(source),
            PistonError::CreateFileError { source, .. } => Some(source),
            PistonError::CreateDirAllError { source, .. } => Some(source),
            PistonError::OpenImageError { source, .. } => Some(source),
            _ => None,
        }
    }
}