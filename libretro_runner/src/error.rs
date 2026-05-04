#[derive(thiserror::Error, Debug)]
pub enum LibretroError {
    #[error("Failed to load library: {0}")]
    LibraryLoad(#[from] libloading::Error),
    #[error("Failed to load game: {0}")]
    GameLoad(String),
    #[error("Audio initialisation failed: {0}")]
    AudioInit(String),
    #[error("Error parsing libretro info: {0}")]
    LibretroInfoParserError(String),
}
