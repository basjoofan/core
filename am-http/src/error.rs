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
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::InvalidUrl(string) => fmt.write_str(string),
                    $(
                        Error::$name => fmt.write_str($description),
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
    EmptyUrl => "empty url",
}
