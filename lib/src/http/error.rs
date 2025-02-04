macro_rules! http_error {
    ($($name: ident => $description: expr,)+) => {
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum Error {
            ReadFailed(std::io::Error),
            WriteFailed(std::io::Error),
            $(
                $name,
            )+
        }

        impl std::error::Error for Error {}

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
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
    InvalidUrlHost => "invalid url host",
    HostNotFound => "host not found",
    TcpConnectFailed => "tcp connect failed",
    NoConnectionAvailable => "no connection available",
    TlsHandshakeFailed => "tls handshake failed",
    WriteFlushFailed => "write flush failed",
    SetReadTimeoutFailed => "set read timeout failed",
}
