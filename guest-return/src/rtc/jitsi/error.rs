// Copyright 2024 Amagi Poland
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub struct Error{
    inner: String
}

impl Error {
    pub fn new(msg: &str) -> Error {
        Error{inner: msg.to_string()}
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        &self.inner
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error obtaining data from backend")
    }

}