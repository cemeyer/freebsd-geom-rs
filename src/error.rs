use std;
use strum_macros::AsRefStr;

/// Wrapped error sources for the geom crate.
#[derive(Debug, AsRefStr)]
pub enum Error {
    Sysctl(sysctl::SysctlError),
    Decode(quick_xml::DeError),
    Parse(strum::ParseError),
    Scan(scan_fmt::parse::ScanError),
    /// Some internal graph invariant was violated.
    GraphError,
}

impl std::convert::From<sysctl::SysctlError> for Error {
    fn from(err: sysctl::SysctlError) -> Error {
        Self::Sysctl(err)
    }
}

impl std::convert::From<quick_xml::DeError> for Error {
    fn from(err: quick_xml::DeError) -> Error {
        Self::Decode(err)
    }
}

impl std::convert::From<strum::ParseError> for Error {
    fn from(err: strum::ParseError) -> Error {
        Self::Parse(err)
    }
}

impl std::convert::From<scan_fmt::parse::ScanError> for Error {
    fn from(err: scan_fmt::parse::ScanError) -> Error {
        Self::Scan(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())?;
        return match self {
            Self::Sysctl(e) => write!(f, ": {}", e),
            Self::Decode(e) => write!(f, ": {}", e),
            Self::Parse(e) => write!(f, ": {}", e),
            Self::Scan(e) => write!(f, ": {}", e),
            Self::GraphError => Ok(()),
        };
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod test {
    use crate::Error;

    #[test]
    fn display_basic() {
        assert_eq!(format!("{}", Error::GraphError), "GraphError");
    }
}
