use std::ffi::NulError;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Cstring(NulError),
    TCPSocket,
    Status(i32),
    Decode,
    Reply,
    Frame,
}


impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.description())?;
        Ok(())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        use self::Error::*;
        match *self {
            Cstring(ref e) => e.description(),
            Status(ref code) => "creat TCP socket status",
            TCPSocket => "creat TCP socket error",
            Reply => "Reply error",
            Decode => "Decode error",
            Frame => "Frame error",
        }
    }
}


impl From<NulError> for Error {
    fn from(error: NulError) -> Self {
        Error::Cstring(error)
    }
}
