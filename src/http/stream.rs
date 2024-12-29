use super::Error;
use super::Scheme;
use super::Url;
use std::io::Read;
use std::io::Write;
use std::iter::{self, FusedIterator};
use std::net::SocketAddr;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

const RACE: Duration = Duration::from_millis(200);

pub enum Stream {
    Plain {
        stream: TcpStream,
        resolve: Duration,
    },
    Cipher {
        stream: TcpStream,
        resolve: Duration,
        attach: Box<rustls::ClientConnection>,
    },
    #[cfg(test)]
    Mock(std::io::Cursor<Vec<u8>>),
}

impl Stream {
    pub fn connect(url: &Url, connect_tiomeout: Option<Duration>, read_tiomeout: Option<Duration>) -> Result<Self, Error> {
        let host = url.host.as_str();
        let port = url.port;

        match url.scheme {
            Scheme::Http => {
                let (stream, resolve) = Self::connect_tcp(host, port, connect_tiomeout, read_tiomeout)?;
                Ok(Stream::Plain { stream, resolve })
            }
            Scheme::Https => Self::connect_tls(host, port, connect_tiomeout, read_tiomeout),
        }
    }

    fn connect_tcp(
        host: &str,
        port: u16,
        connect_tiomeout: Option<Duration>,
        read_tiomeout: Option<Duration>,
    ) -> Result<(TcpStream, Duration), Error> {
        let resolve_start = Instant::now();
        let addrs = match (host, port).to_socket_addrs() {
            Ok(addrs) => Ok(addrs.collect::<Vec<SocketAddr>>()),
            Err(_) => Err(Error::HostNotFound),
        }?;
        let resolve_duration = resolve_start.elapsed();
        let stream = Self::connect_happy(addrs, connect_tiomeout)?;
        stream.set_read_timeout(read_tiomeout).map_err(|_e| Error::SetReadTimeoutFailed)?;
        Ok((stream, resolve_duration))
    }

    fn connect_happy(addrs: Vec<SocketAddr>, timeout: Option<Duration>) -> Result<TcpStream, Error> {
        if let [addr] = &addrs[..] {
            // debug!("DNS returned only one address, using fast path");
            return Self::connect_timeout(addr, timeout);
        }

        // Happy Eyeballs (also called Fast Fallback)
        let (a, b): (Vec<SocketAddr>, Vec<SocketAddr>) = addrs.into_iter().partition(|a| a.is_ipv6());
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut error = None;
        // let start = Instant::now();

        let mut handle_result = |_addr, result| match result {
            Ok(stream) => {
                // debug!("successfully connected to {}, took {:?}", addr, start.elapsed());
                Some(stream)
            }
            Err(e) => {
                // debug!("failed to connect to {}: {}", addr, e);
                if error.is_none() {
                    error = Some(e);
                }
                None
            }
        };

        // This loop will race each connection attempt against others,
        // returning early if a connection attempt is successful.
        for addr in rotate(a.into_iter(), b.into_iter()) {
            let sender = sender.clone();
            std::thread::spawn(move || {
                // debug!("trying to connect to {}", addr);
                let _ = sender.send((addr, Self::connect_timeout(&addr, timeout)));
            });
            if let Ok((addr, result)) = receiver.recv_timeout(RACE) {
                if let Some(stream) = handle_result(addr, result) {
                    return Ok(stream);
                }
            }
        }

        // We must drop this handle to the sender in order to properly disconnect the channel
        // when all the threads are finished.
        drop(sender);

        // This loop waits for replies from the background threads. It will automatically timeout
        // when the background threads' connection attempts timeout and the senders are dropped.
        // This loop is reached when some of the threads do not complete within the race delay.
        for (addr, result) in receiver.iter() {
            if let Some(stream) = handle_result(addr, result) {
                return Ok(stream);
            }
        }

        // debug!("could not connect to any address, took {}", start.elapsed());
        Err(error.unwrap_or(Error::NoConnectionAvailable))
    }

    fn connect_timeout(addr: &SocketAddr, timeout: Option<Duration>) -> Result<TcpStream, Error> {
        match timeout {
            Some(timeout) => TcpStream::connect_timeout(addr, timeout),
            None => TcpStream::connect(addr),
        }
        .map_err(|_e| Error::TcpConnectFailed)
    }

    fn connect_tls(host: &str, port: u16, connect_tiomeout: Option<Duration>, read_tiomeout: Option<Duration>) -> Result<Self, Error> {
        let store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.into(),
        };
        let config = Arc::new(rustls::ClientConfig::builder().with_root_certificates(store).with_no_client_auth());
        let name = host.to_owned().try_into().map_err(|_e| Error::InvalidUrlHost)?;
        let attach = Box::new(rustls::ClientConnection::new(config, name).map_err(|_e| Error::TlsHandshakeFailed)?);
        let (stream, resolve) = Self::connect_tcp(host, port, connect_tiomeout, read_tiomeout)?;
        Ok(Stream::Cipher { attach, stream, resolve })
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

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain { stream, .. } => stream.read(buf),
            Stream::Cipher {
                ref mut stream, attach, ..
            } => {
                match rustls::Stream::new(attach.as_mut(), stream).read(buf) {
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                        attach.send_close_notify();
                        attach.complete_io(stream)?;
                        Ok(0)
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        // In some cases the server does not terminate the connection cleanly
                        // We just turn that error into EOF.
                        Ok(0)
                    }
                    r => r,
                }
            }
            #[cfg(test)]
            Stream::Mock(cursor) => cursor.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain { stream, .. } => stream.write(buf),
            Stream::Cipher { stream, attach, .. } => rustls::Stream::new(attach.as_mut(), stream).write(buf),
            #[cfg(test)]
            Stream::Mock(cursor) => cursor.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Plain { stream, .. } => stream.flush(),
            Stream::Cipher { stream, attach, .. } => rustls::Stream::new(attach.as_mut(), stream).flush(),
            #[cfg(test)]
            Stream::Mock(cursor) => cursor.flush(),
        }
    }
}

fn rotate<T, A: FusedIterator<Item = T>, B: FusedIterator<Item = T>>(mut a: A, mut b: B) -> impl Iterator<Item = T> {
    let mut c = None;

    iter::from_fn(move || {
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
