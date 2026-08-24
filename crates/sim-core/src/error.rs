use std::fmt;

#[derive(Debug)]
pub enum SimError {
    Config(String),
    Io(std::io::Error),
    Checkpoint(String),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::Config(msg) => write!(f, "config error: {msg}"),
            SimError::Io(err) => write!(f, "io error: {err}"),
            SimError::Checkpoint(msg) => write!(f, "checkpoint error: {msg}"),
        }
    }
}

impl std::error::Error for SimError {}

impl From<std::io::Error> for SimError {
    fn from(value: std::io::Error) -> Self {
        SimError::Io(value)
    }
}
