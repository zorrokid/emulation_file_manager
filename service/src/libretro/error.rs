use std::fmt::Display;

#[derive(Debug)]
pub enum LibretroPreflightError {
    UnsupportedExtension(String),
    DownloadError(String),
    NoFileInFileSet,
    SystemDirNotSet,
    FirmwareNotAvailable(String),
    InvalidInitialFile(String),
    CoreDirNotSet,
    CoreNotRecognized(String),
    InfoParseError(String),
}

impl Display for LibretroPreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LibretroPreflightError::UnsupportedExtension(ext) => {
                write!(
                    f,
                    "The file extension '{ext}' is not supported by the selected core."
                )
            }
            LibretroPreflightError::DownloadError(msg) => {
                write!(f, "Failed to download ROM: {msg}")
            }
            LibretroPreflightError::NoFileInFileSet => {
                write!(f, "The selected file set does not contain any files.")
            }
            LibretroPreflightError::SystemDirNotSet => {
                write!(f, "The system directory is not configured in settings.")
            }
            LibretroPreflightError::FirmwareNotAvailable(desc) => {
                write!(f, "Required firmware not available: {desc}")
            }
            LibretroPreflightError::InvalidInitialFile(file) => {
                write!(
                    f,
                    "The specified initial file '{file}' was not found in the file set."
                )
            }
            LibretroPreflightError::CoreDirNotSet => {
                write!(
                    f,
                    "The libretro core directory is not configured in settings."
                )
            }
            LibretroPreflightError::CoreNotRecognized(core) => {
                write!(
                    f,
                    "The specified core '{core}' was not recognized in the core info."
                )
            }
            LibretroPreflightError::InfoParseError(msg) => {
                write!(f, "Failed to parse core info: {msg}")
            }
        }
    }
}
