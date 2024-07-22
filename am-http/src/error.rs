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
            $(
                $name,
            )+
        }
        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::InvalidUrl(string) => f.write_str(string),
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
    InvalidMethod => "invalid method",
    InvalidVersion => "invalid version",
    InvalidStatus => "invalid status",
    EmptyUrl => "empty url",
    EmptyUrlHost => "empty url host",
    InvalidUrlHost => "invalid url host",
    InvalidUrlPort => "invalid url port",
    InvalidUrlScheme => "invalid url scheme",
    HostNotFound => "host not found",
    TcpConnectFailed => "tcp connect failed",
    NoConnectionAvailable => "no connection available",
    TlsHandshakeFailed => "tls handshake failed",
    WriteFailed => "write failed",
    WriteFlushFailed => "write flush failed",
    SetReadTimeoutFailed => "set read timeout failed",
    MultipartPrepareFailed => "multipart prepare failed",
    ReadFailed => "read failed",
}
