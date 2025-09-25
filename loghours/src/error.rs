/*!
 * Enum allowing a hidden terminal cursor to be re-shown
 * after error caught and before loghour quits
 */

use std::io;
use anyhow;
use chrono::format::ParseError;

use crate::util::show_cursor;

#[derive(Debug)]
#[allow(dead_code)]
pub enum CustomError {
    Os(anyhow::Error),
    Io(io::Error),
    Parse(ParseError),
}

impl From<anyhow::Error> for CustomError {
    fn from(err: anyhow::Error) -> Self {
        show_cursor().unwrap();
        CustomError::Os(err)
    }
}

impl From<io::Error> for CustomError {
    fn from(err: io::Error) -> Self {
        show_cursor().unwrap();
        CustomError::Io(err)
    }
}

impl From<ParseError> for CustomError {
    fn from(err: ParseError) -> Self {
        CustomError::Parse(err)
    }
}
