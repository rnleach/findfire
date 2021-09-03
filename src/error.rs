use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug, Clone, Copy)]
pub struct FindFireError {
    pub msg: &'static str,
}

impl Display for FindFireError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.msg)
    }
}

impl Error for FindFireError {}

#[derive(Debug, Clone, Copy)]
pub struct ConnectFireError {
    pub msg: &'static str,
}

impl Display for ConnectFireError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.msg)
    }
}

impl Error for ConnectFireError {}
