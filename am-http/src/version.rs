use crate::Error;

pub enum Version {
    Http09,
    Http10,
    Http11,
    Http20,
    Http30,
}

/// Converts an &str to an http version.
impl std::convert::TryFrom<&str> for Version {
    type Error = Error;
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        match str {
            "HTTP/0.9" => Ok(Version::Http09),
            "HTTP/1.0" => Ok(Version::Http10),
            "HTTP/1.1" => Ok(Version::Http11),
            "HTTP/2.0" => Ok(Version::Http20),
            "HTTP/3.0" => Ok(Version::Http30),
            _ => Err(Error::InvalidVersion),
        }
    }
}

/// Return a &str representation of the http version
impl std::convert::AsRef<str> for Version {
    fn as_ref(&self) -> &str {
        match self {
            Version::Http09 => "HTTP/0.9",
            Version::Http10 => "HTTP/1.0",
            Version::Http11 => "HTTP/1.1",
            Version::Http20 => "HTTP/2.0",
            Version::Http30 => "HTTP/3.0",
        }
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl Default for Version {
    fn default() -> Version {
        Version::Http11
    }
}
