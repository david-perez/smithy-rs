use crate::BoxError;
use std::{error::Error as StdError, fmt};

/// Errors that can happen when using `aws-smithy-server`.
#[derive(Debug)]
pub struct Error {
    inner: BoxError,
}

impl Error {
    pub(crate) fn new(error: impl Into<BoxError>) -> Self {
        Self {
            inner: error.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&*self.inner)
    }
}
