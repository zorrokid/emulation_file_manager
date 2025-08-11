use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone)]
pub enum Error {
    DbError(String),
    DeserializationError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Error::DbError(message) => write!(f, "Database error: {}", message),
            Error::DeserializationError(message) => write!(f, "Deserialization error: {}", message),
        }
    }
}

impl From<database::database_error::DatabaseError> for Error {
    fn from(err: database::database_error::DatabaseError) -> Self {
        Error::DbError(err.to_string())
    }
}
