use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IxodesError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("ICO error: {0}")]
    Ico(String),

    #[error("Windows API error: {0}")]
    Windows(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Resource error: {0}")]
    Resource(String),

    #[error("General error: {0}")]
    General(String),
}

impl Serialize for IxodesError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
