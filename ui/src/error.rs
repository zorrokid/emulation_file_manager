use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone)]
pub enum Error {
    DbError(String),
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Error::DbError(message) => write!(f, "Database error: {}", message),
        }
    }
}
