#[derive(Debug)]
pub enum Error {
    InvalidUrlHost,
    HostNotFound(std::io::Error),
    ConnectFailed(std::io::Error),
    ConnectTimeout,
    NoConnectionAvailable,
    TlsHandshakeFailed,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidUrlHost => write!(f, "invalid url host"),
            Error::HostNotFound(error) => write!(f, "host not found: {error}"),
            Error::ConnectFailed(error) => write!(f, "connect failed: {error}"),
            Error::ConnectTimeout => write!(f, "connect timeout"),
            Error::NoConnectionAvailable => write!(f, "no connection available"),
            Error::TlsHandshakeFailed => write!(f, "tls handshake failed"),
        }
    }
}
