impl std::error::Error for Error {}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Error {
        Error::InvalidUrl(error.to_string())
    }
}

macro_rules! http_error {
    ($($name: ident => $description: expr,)+) => {
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum Error {
            InvalidUrl(String),
            ReadFailed(std::io::Error),
            WriteFailed(std::io::Error),
            $(
                $name,
            )+
        }
        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::InvalidUrl(string) => f.write_str(string),
                    Error::ReadFailed(error) => error.fmt(f),
                    Error::WriteFailed(error) => error.fmt(f),
                    $(
                        Error::$name => f.write_str($description),
                    )+
                }
            }
        }
    }
}

http_error! {
    EmptyMessage => "empty message",
    EmptyUrl => "empty url",
    EmptyUrlHost => "empty url host",
    InvalidUrlHost => "invalid url host",
    InvalidUrlPort => "invalid url port",
    InvalidUrlScheme => "invalid url scheme",
    HostNotFound => "host not found",
    TcpConnectFailed => "tcp connect failed",
    NoConnectionAvailable => "no connection available",
    TlsHandshakeFailed => "tls handshake failed",
    WriteFlushFailed => "write flush failed",
    SetReadTimeoutFailed => "set read timeout failed",
    MultipartPrepareFailed => "multipart prepare failed",
}
