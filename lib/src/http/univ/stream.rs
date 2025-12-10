use super::super::Error;
use super::super::Scheme;
use super::super::Url;
use rustls_platform_verifier::ConfigVerifierExt;
use std::iter::FusedIterator;
use std::iter::from_fn;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
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
use tokio::task::JoinSet;
use tokio::time;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::ClientConfig;

pub enum Stream {
    Plain {
        stream: TcpStream,
        resolve: Duration,
    },
    Cipher {
        stream: Box<TlsStream<TcpStream>>,
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
        let addrs = match (host, port).to_socket_addrs() {
            Ok(addrs) => Ok(addrs.collect::<Vec<SocketAddr>>()),
            Err(error) => Err(Error::HostNotFound(error)),
        }?;
        let resolve = resolve.elapsed();
        match time::timeout(tiomeout, Self::connect_happy(addrs)).await {
            Ok(Some(Ok(stream))) => Ok((stream, resolve)),
            Ok(Some(Err(error))) => Err(Error::ConnectFailed(error)),
            Ok(None) => Err(Error::NoConnectionAvailable),
            Err(_) => Err(Error::ConnectTimeout),
        }
    }

    async fn connect_tls(host: &str, port: u16, tiomeout: Duration) -> Result<Self, Error> {
        let config = ClientConfig::with_platform_verifier().map_err(|_e| Error::InvalidUrlHost)?;
        let connector = TlsConnector::from(Arc::new(config));
        let domain = host.to_owned().try_into().map_err(|_e| Error::InvalidUrlHost)?;
        let (stream, resolve) = Self::connect_tcp(host, port, tiomeout).await?;
        let stream = Box::new(connector.connect(domain, stream).await.map_err(|_e| Error::TlsHandshakeFailed)?);
        Ok(Stream::Cipher { stream, resolve })
    }

    async fn connect_happy(addrs: Vec<SocketAddr>) -> Option<std::io::Result<TcpStream>> {
        if let [addr] = &addrs[..] {
            // DNS returned only one address, using fast path
            return Some(TcpStream::connect(addr).await);
        }
        // Happy Eyeballs (also called Fast Fallback)
        let (a, b): (Vec<SocketAddr>, Vec<SocketAddr>) = addrs.into_iter().partition(|a| a.is_ipv6());
        let mut error = None;
        // This loop will race each connection attempt against others,
        // returning early if a connection attempt is successful.
        let mut set = JoinSet::new();
        for addr in rotate(a.into_iter(), b.into_iter()) {
            set.spawn(TcpStream::connect(addr));
        }
        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(s)) => return Some(Ok(s)),
                Ok(Err(e)) => error = Some(Err(e)),
                Err(e) => error = Some(Err(e.into())),
            }
        }
        error
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

fn rotate<T, A: FusedIterator<Item = T>, B: FusedIterator<Item = T>>(mut a: A, mut b: B) -> impl Iterator<Item = T> {
    let mut c = None;
    from_fn(move || {
        if let Some(b) = c.take() {
            return Some(b);
        }
        match (a.next(), b.next()) {
            (Some(a), Some(b)) => {
                c = Some(b);
                Some(a)
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    })
}

#[test]
fn test_rotate_even() {
    let x: Vec<u32> = rotate(vec![1, 2, 3].into_iter(), vec![4, 5, 6].into_iter()).collect();
    assert_eq!(&x[..], &[1, 4, 2, 5, 3, 6][..]);
}

#[test]
fn test_rotate_left() {
    let x: Vec<u32> = rotate(vec![1, 2, 3, 100, 101].into_iter(), vec![4, 5, 6].into_iter()).collect();
    assert_eq!(&x[..], &[1, 4, 2, 5, 3, 6, 100, 101][..]);
}

#[test]
fn test_rotate_right() {
    let x: Vec<u32> = rotate(vec![1, 2, 3].into_iter(), vec![4, 5, 6, 100, 101].into_iter()).collect();
    assert_eq!(&x[..], &[1, 4, 2, 5, 3, 6, 100, 101][..]);
}

#[tokio::test]
async fn test_connect() {
    crate::tests::start_server(30000).await;
    let stream = Stream::connect(&Url::from("http://127.0.0.1:30000/get"), Duration::from_secs(2)).await;
    if let Err(error) = stream.as_ref() {
        println!("{error:?}");
    }
    assert!(stream.is_ok());
    let stream = Stream::connect(&Url::from("http://localhost:30000/get"), Duration::from_secs(2)).await;
    if let Err(error) = stream.as_ref() {
        println!("{error:?}");
    }
    assert!(stream.is_ok());
    let stream = Stream::connect(&Url::from("http://localhost:88/get"), Duration::from_secs(2)).await;
    assert!(stream.is_err());
    if let Err(error) = stream {
        println!("{error:?}");
    }
}
