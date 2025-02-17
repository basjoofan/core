use super::Error;
use super::Scheme;
use super::Url;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::ReadBuf;
use tokio::net::TcpStream;
use tokio::time;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls;

pub enum Stream {
    Plain {
        stream: TcpStream,
        resolve: Duration,
    },
    Cipher {
        stream: TlsStream<TcpStream>,
        resolve: Duration,
    },
    #[cfg(test)]
    Mock(std::io::Cursor<Vec<u8>>),
}

impl Stream {
    pub async fn connect(url: &Url, tiomeout: Duration) -> Result<Self, Error> {
        let host = url.host.as_str();
        let port = url.port;

        match url.scheme {
            Scheme::Http => {
                let (stream, resolve) = Self::connect_tcp(host, port, tiomeout).await?;
                Ok(Stream::Plain { stream, resolve })
            }
            Scheme::Https => Self::connect_tls(host, port, tiomeout).await,
        }
    }

    async fn connect_tcp(host: &str, port: u16, tiomeout: Duration) -> Result<(TcpStream, Duration), Error> {
        let resolve = Instant::now();
        match time::timeout(tiomeout, TcpStream::connect((host, port))).await {
            Ok(Ok(stream)) => Ok((stream, resolve.elapsed())),
            Ok(Err(error)) => Err(Error::ConnectFailed(error)),
            Err(error) => Err(Error::ConnectFailed(std::io::Error::from(error))),
        }
    }

    async fn connect_tls(host: &str, port: u16, tiomeout: Duration) -> Result<Self, Error> {
        use rustls_platform_verifier::BuilderVerifierExt;
        let config = rustls::ClientConfig::builder().with_platform_verifier().with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
        let domain = host.to_owned().try_into().map_err(|_e| Error::InvalidUrlHost)?;
        let (stream, resolve) = Self::connect_tcp(host, port, tiomeout).await?;
        let stream = connector.connect(domain, stream).await.map_err(|_e| Error::TlsHandshakeFailed)?;
        Ok(Stream::Cipher { stream, resolve })
    }

    pub fn resolve(&self) -> Duration {
        match self {
            Stream::Plain { resolve, .. } => *resolve,
            Stream::Cipher { resolve, .. } => *resolve,
            #[cfg(test)]
            Stream::Mock(_) => Duration::default(),
        }
    }
}

impl AsyncRead for Stream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Stream::Plain { stream, .. } => Pin::new(stream).poll_read(cx, buf),
            Stream::Cipher { stream, .. } => match Pin::new(&mut *stream).poll_read(cx, buf) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => Poll::Ready(Ok(())),
                Poll::Ready(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionAborted => Pin::new(stream).poll_shutdown(cx),
                Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                Poll::Pending => Poll::Pending,
            },
            #[cfg(test)]
            Stream::Mock(cursor) => Pin::new(cursor).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            Stream::Plain { stream, .. } => Pin::new(stream).poll_write(cx, buf),
            Stream::Cipher { stream, .. } => Pin::new(stream).poll_write(cx, buf),
            #[cfg(test)]
            Stream::Mock(cursor) => Pin::new(cursor).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            Stream::Plain { stream, .. } => Pin::new(stream).poll_flush(cx),
            Stream::Cipher { stream, .. } => Pin::new(stream).poll_flush(cx),
            #[cfg(test)]
            Stream::Mock(cursor) => Pin::new(cursor).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            Stream::Plain { stream, .. } => Pin::new(stream).poll_shutdown(cx),
            Stream::Cipher { stream, .. } => Pin::new(stream).poll_shutdown(cx),
            #[cfg(test)]
            Stream::Mock(cursor) => Pin::new(cursor).poll_shutdown(cx),
        }
    }
}

#[tokio::test]
async fn test_connect() {
    let stream = Stream::connect(&Url::from("http://httpbin.org/get"), Duration::from_secs(2)).await;
    assert!(stream.is_ok());
    let stream = Stream::connect(&Url::from("https://httpbin.org/get"), Duration::from_secs(2)).await;
    assert!(stream.is_ok());
    let stream = Stream::connect(&Url::from("http://httpbin.org:80/get"), Duration::from_secs(2)).await;
    assert!(stream.is_ok());
    let stream = Stream::connect(&Url::from("https://httpbin.org:443/get"), Duration::from_secs(2)).await;
    assert!(stream.is_ok());
    let stream = Stream::connect(&Url::from("http://httpbin.org:88/get"), Duration::from_secs(2)).await;
    assert!(stream.is_err());
    if let Err(error) = stream {
        println!("{:?}", error);
    }
}
