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

    ProtoLinkError(String),

    UnsupportedTargetError(String),

    XcodeInstallError(String),

    XcodeSelectInstallError(String),

    XcodeSelectPathingError(String),

    XcodeBuildError(String),

    IOSSdkMissingError(String),

    ADBDevicesError(String),

    ParseUTF8Error(String),

    XcrunDevicectlError(String),

    KeyChainUnlockError(String),

    KeyChainImportError(String),

    IntoJSONError(String),

    OpenSSLKeyGenError(String),
    
    OpenSSLCSRError(String),

    ReadCSRError(String),

    Base64DecodeError(String),

    ASCClientParseEncodingKeyError(String),

    ASCClientJWTEncodeError(String),

    WritePlUtilError(String),

    XcrunInstallError(String),

    XcrunLaunchError(String),

    CodesignError(String),

    DeviceProvisionError(String),

    SecurityFindIdentityError(String),

    PlutilConvertError(String),

    ExtractAPKError(String),

    InstallAPKError(String),

    RunAPKError(String),

    WhoAmIError(String),

    KeyToolError(String),

    APKSignerError(String),

    UnsupportedOSError{ os: String, target: String },

    CopyFileError{ input_path: PathBuf, output_path: PathBuf, source: IoError },

    LipoError{ first_binary: PathBuf, second_binary:PathBuf, source: String },

    MacOSIconError{ input_path: PathBuf, output_path: PathBuf, source: IoError },

    ReadDirError{ path: PathBuf, source: IoError },

    MapDirError{ path: PathBuf, source: IoError },

    QueryProvisionError{ path: PathBuf, source: IoError },

    RemoveSubdirError{ path: PathBuf, source: IoError },

    RemoveFileError{ path: PathBuf, source: IoError },

    CreateFileError{ path: PathBuf, source: IoError },

    RenameFileError{ path: PathBuf, source: IoError },

    CreateDirAllError{ path: PathBuf, source: IoError },

    OpenImageError{ path: PathBuf, source: ImageError },

    ASCClientUreqError{ endpoint: String, e: String },

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
            PistonError::ProtoLinkError(err) => write!(f, "Failed to proto link android build directory: {}", err),
            PistonError::UnsupportedTargetError(err) => write!(f, "User Specified Target is Unsupported: {}", err),
            PistonError::XcodeInstallError(err) => write!(f, "Xcode Installation not Found: {}", err),
            PistonError::XcodeSelectInstallError(err) => write!(f, "Xcode-select Installation verification failed: {}", err),
            PistonError::XcodeSelectPathingError(err) => write!(f, "Xcode-select pathing does not match expected value: {}", err),
            PistonError::XcodeBuildError(err) => write!(f, "Xcodebuild missing iOS sdk: {}", err),
            PistonError::IOSSdkMissingError(err) => write!(f, "IOS SDK missing from xcodebuild sdks: {}", err),
            PistonError::ADBDevicesError(err) => write!(f, "Error Running 'ADB Devices', check installation and .env path: {}", err),
            PistonError::ParseUTF8Error(err) => write!(f, "Error Parsing UTF8: {}", err),
            PistonError::XcrunDevicectlError(err) => write!(f, "Failed to run devicectl command. Ensure Xcode 15+ is installed: {}", err),
            PistonError::KeyChainUnlockError(err) => write!(f, "Failed to unlock the security keychain: {}", err),
            PistonError::KeyChainImportError(err) => write!(f, "Failed to import profile to security keychain: {}", err),
            PistonError::IntoJSONError(err) => write!(f, "Error converting into JSON: {}", err),
            PistonError::OpenSSLKeyGenError(err) => write!(f, "Error Generating New Key with openssl: {}", err),
            PistonError::OpenSSLCSRError(err) => write!(f, "Error Generating CSR with openssl: {}", err),
            PistonError::ReadCSRError(err) => write!(f, "Error Reading CSR: {}", err),
            PistonError::Base64DecodeError(err) => write!(f, "Error Decoding Base 64: {}", err),
            PistonError::ASCClientParseEncodingKeyError(err) => write!(f, "Error parsing encoding key: {}", err),
            PistonError::ASCClientJWTEncodeError(err) => write!(f, "Error encoding JWT: {}", err),
            PistonError::WritePlUtilError(err) => write!(f, "Failed to write Plutil contents: {}", err),
            PistonError::XcrunInstallError(err) => write!(f, "Error installing app bundle to target device via xcrun: {}", err),
            PistonError::XcrunLaunchError(err) => write!(f, "Error launching app bundle on target device via xcrun: {}", err),
            PistonError::CodesignError(err) => write!(f, "Error signing app bundle with codesign: {}", err),
            PistonError::DeviceProvisionError(err) => write!(f, "Error provisioning target device with idp: {}", err),
            PistonError::SecurityFindIdentityError(err) => write!(f, "Error checking for local identities with `security find-identity -v -p codesigning`: {}", err),
            PistonError::PlutilConvertError(err) => write!(f, "Error using plutil to convert filetype`: {}", err),
            PistonError::ExtractAPKError(err) => write!(f, "Error extracting APK from AAB with bundletool: {}", err),
            PistonError::InstallAPKError(err) => write!(f, "Error Installing APK with bundletool: {}", err),
            PistonError::RunAPKError(err) => write!(f, "Error Running APK with ADB: {}", err),
            PistonError::WhoAmIError(err) => write!(f, "Error running 'whoami': {}", err),
            PistonError::KeyToolError(err) => write!(f, "Error running 'keytool': {}", err),
            PistonError::APKSignerError(err) => write!(f, "Error running 'apksigner': {}", err),
            PistonError::UnsupportedOSError{ os, target, .. } => write!(f, "Host system: {:?} does not support the target: {:?}", os, target),
            PistonError::LipoError{ first_binary, second_binary, source } => write!(f, "Error making universal binary with Lipo first binary: {:?}, second binary: {:?}, source: {}", first_binary, second_binary, source),
            PistonError::CopyFileError { input_path, output_path, .. } => write!(f, "Failed to copy {:?} to {:?}", input_path, output_path),
            PistonError::MacOSIconError { input_path, output_path, .. } => write!(f, "Failed to format icon {:?} to {:?}", input_path, output_path),
            PistonError::ReadDirError { path, .. } => write!(f, "Failed to read directory {:?}", path),
            PistonError::MapDirError { path, .. } => write!(f, "Failed to map directory contents {:?}", path),
            PistonError::QueryProvisionError { path, .. } => write!(f, "Failed to query the security provision profile {:?}", path),
            PistonError::RemoveSubdirError { path, .. } => write!(f, "Failed to remove subdirectory {:?}", path),
            PistonError::RemoveFileError { path, .. } => write!(f, "Failed to remove file {:?}", path),
            PistonError::CreateFileError { path, .. } => write!(f, "Failed to Create file {:?}", path),
            PistonError::RenameFileError { path, .. } => write!(f, "Failed to Rename file {:?}", path),
            PistonError::CreateDirAllError { path, .. } => write!(f, "Failed to create dir all {:?}", path),
            PistonError::OpenImageError { path, .. } => write!(f, "Failed to open image {:?}", path),
            PistonError::ASCClientUreqError { endpoint, e } => write!(f, "ASC API error at endpoint: {:?}, Error message: {}", endpoint, e),
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
            PistonError::MapDirError { source, .. } => Some(source),
            PistonError::QueryProvisionError { source, .. } => Some(source),
            PistonError::RemoveSubdirError { source, .. } => Some(source),
            PistonError::RemoveFileError { source, .. } => Some(source),
            PistonError::CreateFileError { source, .. } => Some(source),
            PistonError::CreateDirAllError { source, .. } => Some(source),
            PistonError::RenameFileError { source, .. } => Some(source),
            PistonError::OpenImageError { source, .. } => Some(source),
            _ => None,
        }
    }
}