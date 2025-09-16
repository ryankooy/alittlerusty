use std::io;
use anyhow;

use crate::util::show_cursor;

#[derive(Debug)]
#[allow(dead_code)]
pub enum CustomError {
    AnyhowError(anyhow::Error),
    IoError(io::Error),
}

impl From<anyhow::Error> for CustomError {
    fn from(err: anyhow::Error) -> Self {
        show_cursor().unwrap();
        CustomError::AnyhowError(err)
    }
}

impl From<io::Error> for CustomError {
    fn from(err: io::Error) -> Self {
        show_cursor().unwrap();
        CustomError::IoError(err)
    }
}
