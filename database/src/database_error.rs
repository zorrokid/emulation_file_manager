use std::fmt::{Display, Formatter};

use sqlx::Error as SqlxError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    SqlxError(#[from] SqlxError),

    #[error("Cannot delete because entity is in use")]
    InUse,
    #[error("Database error")]
    DbError(String),
    #[error("Parse error: {0}")]
    SerializationError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl PartialEq for DatabaseError {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    InUse,
    DbError(String),
    ParseError(String),
    DecodeError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InUse => write!(f, "Cannot delete because entity is in use"),
            Error::DbError(err) => write!(f, "Database error: {}", err),
            Error::ParseError(err) => write!(f, "Parse error: {}", err),
            Error::DecodeError(err) => write!(f, "Decode error: {}", err),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::DbError(err.to_string())
    }
}
