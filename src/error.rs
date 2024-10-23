use sqlx::Error as SqlxError;
use std::path::PathBuf;
use thiserror::Error;

// Define your custom error type
#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLx Error Occurred: {source}")]
    SqlxError {
        #[from]
        source: SqlxError,
    },

    #[error("audio folder is missing from the current directory: {0}")]
    MissingAudioFolder(PathBuf),
    #[error("entries.db is missing from the audio folder")]
    MissingEntriesDB,
}
