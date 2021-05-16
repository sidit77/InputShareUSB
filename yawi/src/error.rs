use std::{fmt, error};
use winapi::um::errhandlingapi::GetLastError;

/// Windows error code.
///
/// See [System Error Codes](https://msdn.microsoft.com/en-us/library/windows/desktop/ms681381.aspx) for more information.
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error(u32);

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub const SUCCESS: Error = Error(0);
}

impl Error {
    /// Returns true if this is the success error code.
    pub const fn is_success(self) -> bool {
        self.0 == 0
    }
    /// Gets the last error code.
    ///
    /// See [GetLastError function](https://msdn.microsoft.com/en-us/library/windows/desktop/ms679360.aspx) for more information.
    pub fn last() -> Error {
        Error(unsafe { GetLastError() })
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#X}", self.0)
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ErrorCode({:#X})", self.0)
    }
}
impl error::Error for Error {
    fn description(&self) -> &str {
        "system error code"
    }
}