//! Blocking TCP and WebSocket transports. Same postcard codec on both.

use crate::protocol::{self, CodecError};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, ErrorKind};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;
use tungstenite::protocol::WebSocket;
use tungstenite::{Message, accept as ws_accept, client::client_with_config};

#[derive(Debug)]
pub enum TransportError {
    Io(io::Error),
    Timeout,
    Closed,
    Codec(CodecError),
    Handshake(String),
    BadUrl(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Io(e) => write!(f, "io: {e}"),
            TransportError::Timeout => write!(f, "timeout"),
            TransportError::Closed => write!(f, "connection closed"),
            TransportError::Codec(e) => write!(f, "{e}"),
            TransportError::Handshake(e) => write!(f, "handshake: {e}"),
            TransportError::BadUrl(e) => write!(f, "url: {e}"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<io::Error> for TransportError {
    fn from(e: io::Error) -> Self {
        if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock {
            TransportError::Timeout
        } else if e.kind() == ErrorKind::UnexpectedEof {
            TransportError::Closed
        } else {
            TransportError::Io(e)
        }
    }
}

impl From<CodecError> for TransportError {
    fn from(e: CodecError) -> Self {
        match e {
            CodecError::Io(io) => TransportError::from(io),
            other => TransportError::Codec(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Tcp,
    Ws,
    Unix,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListenUrl {
    pub scheme: Scheme,
    pub addr: SocketAddr,
}

/// `unix:///tmp/agentplace.sock` → `/tmp/agentplace.sock`. Empty and `@abstract` are errors.
pub fn parse_unix_path(s: &str) -> Result<std::path::PathBuf, TransportError> {
    let s = s.trim();
    let Some(rest) = s.strip_prefix("unix://") else {
        return Err(TransportError::BadUrl(format!(
            "expected unix://path, got {s}"
        )));
    };
    if rest.is_empty() {
        return Err(TransportError::BadUrl("unix:// path is empty".into()));
    }
    if rest.starts_with('@') {
        return Err(TransportError::BadUrl(
            "abstract unix sockets are not supported".into(),
        ));
    }
    Ok(std::path::PathBuf::from(rest))
}

pub fn parse_listen_url(s: &str) -> Result<ListenUrl, TransportError> {
    let s = s.trim();
    let (scheme, rest) = if let Some(rest) = s.strip_prefix("tcp://") {
        (Scheme::Tcp, rest)
    } else if let Some(rest) = s.strip_prefix("ws://") {
        (Scheme::Ws, rest)
    } else if s.starts_with("wss://") {
        return Err(TransportError::BadUrl(
            "wss/TLS is not supported in M7".into(),
        ));
    } else {
        return Err(TransportError::BadUrl(format!(
            "expected tcp://host:port or ws://host:port, got {s}"
        )));
    };
    let addr: SocketAddr = rest
        .parse()
        .map_err(|e| TransportError::BadUrl(format!("invalid socket address {rest}: {e}")))?;
    Ok(ListenUrl { scheme, addr })
}

enum ListenInner {
    Tcp {
        listener: TcpListener,
        scheme: Scheme,
    },
    #[cfg(unix)]
    Unix {
        listener: std::os::unix::net::UnixListener,
        path: std::path::PathBuf,
    },
}

pub struct Listener {
    inner: ListenInner,
}

impl Listener {
    pub fn bind(url: &str) -> Result<Self, TransportError> {
        if url.trim().starts_with("unix://") {
            return bind_unix(url);
        }
        let parsed = parse_listen_url(url)?;
        let listener = TcpListener::bind(parsed.addr)?;
        listener.set_nonblocking(false)?;
        Ok(Self {
            inner: ListenInner::Tcp {
                listener,
                scheme: parsed.scheme,
            },
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        match &self.inner {
            ListenInner::Tcp { listener, .. } => Ok(listener.local_addr()?),
            #[cfg(unix)]
            ListenInner::Unix { .. } => Err(TransportError::BadUrl(
                "unix listener has no tcp address".into(),
            )),
        }
    }

    pub fn local_url(&self) -> Result<String, TransportError> {
        match &self.inner {
            ListenInner::Tcp { listener, scheme } => {
                let addr = listener.local_addr()?;
                let name = match scheme {
                    Scheme::Tcp => "tcp",
                    Scheme::Ws => "ws",
                    Scheme::Unix => "unix",
                };
                Ok(format!("{name}://{addr}"))
            }
            #[cfg(unix)]
            ListenInner::Unix { path, .. } => Ok(format!("unix://{}", path.display())),
        }
    }

    pub fn scheme(&self) -> Scheme {
        match &self.inner {
            ListenInner::Tcp { scheme, .. } => *scheme,
            #[cfg(unix)]
            ListenInner::Unix { .. } => Scheme::Unix,
        }
    }

    pub fn accept(&self) -> Result<Connection, TransportError> {
        match &self.inner {
            ListenInner::Tcp { listener, scheme } => {
                let (stream, _) = listener.accept()?;
                Connection::from_accepted(stream, *scheme)
            }
            #[cfg(unix)]
            ListenInner::Unix { listener, .. } => {
                let (stream, _) = listener.accept()?;
                Connection::from_unix(stream)
            }
        }
    }

    /// Returns `Ok(None)` when no connection is pending.
    pub fn accept_nonblocking(&self) -> Result<Option<Connection>, TransportError> {
        match &self.inner {
            ListenInner::Tcp { .. } => match self.accept_tcp_nonblocking()? {
                Some(stream) => Ok(Some(Connection::from_accepted(stream, self.scheme())?)),
                None => Ok(None),
            },
            #[cfg(unix)]
            ListenInner::Unix { listener, .. } => {
                listener.set_nonblocking(true)?;
                let result = listener.accept();
                listener.set_nonblocking(false)?;
                match result {
                    Ok((stream, _)) => Ok(Some(Connection::from_unix(stream)?)),
                    Err(e)
                        if e.kind() == ErrorKind::WouldBlock
                            || e.kind() == ErrorKind::Interrupted =>
                    {
                        Ok(None)
                    }
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    /// TCP accept only. WebSocket handshake is done in `Connection::from_accepted`
    /// so the accept loop can spawn it on a worker thread.
    pub fn accept_tcp_nonblocking(&self) -> Result<Option<TcpStream>, TransportError> {
        let ListenInner::Tcp { listener, .. } = &self.inner else {
            return Err(TransportError::BadUrl(
                "accept_tcp_nonblocking is only for tcp/ws".into(),
            ));
        };
        listener.set_nonblocking(true)?;
        let result = listener.accept();
        listener.set_nonblocking(false)?;
        match result {
            Ok((stream, _)) => Ok(Some(stream)),
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }
}

fn bind_unix(url: &str) -> Result<Listener, TransportError> {
    let path = parse_unix_path(url)?;
    #[cfg(unix)]
    {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        let listener = std::os::unix::net::UnixListener::bind(&path)?;
        listener.set_nonblocking(false)?;
        Ok(Listener {
            inner: ListenInner::Unix { listener, path },
        })
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(TransportError::BadUrl(
            "unix sockets are not supported on this OS".into(),
        ))
    }
}

pub enum Connection {
    Tcp(TcpStream),
    Ws(WebSocket<TcpStream>),
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
}

impl Connection {
    pub fn from_accepted(stream: TcpStream, scheme: Scheme) -> Result<Self, TransportError> {
        stream.set_nodelay(true)?;
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(15)))?;
        stream.set_write_timeout(Some(Duration::from_secs(15)))?;
        match scheme {
            Scheme::Tcp => Ok(Connection::Tcp(stream)),
            Scheme::Ws => {
                let ws = ws_accept(stream).map_err(|e| TransportError::Handshake(e.to_string()))?;
                Ok(Connection::Ws(ws))
            }
            Scheme::Unix => Err(TransportError::BadUrl(
                "unix accept is not a tcp stream".into(),
            )),
        }
    }

    #[cfg(unix)]
    pub fn from_unix(stream: std::os::unix::net::UnixStream) -> Result<Self, TransportError> {
        stream.set_read_timeout(Some(Duration::from_secs(15)))?;
        stream.set_write_timeout(Some(Duration::from_secs(15)))?;
        Ok(Connection::Unix(stream))
    }

    pub fn connect(url: &str) -> Result<Self, TransportError> {
        if url.trim().starts_with("unix://") {
            return connect_unix(url);
        }
        let parsed = parse_listen_url(url)?;
        let stream = TcpStream::connect_timeout(&parsed.addr, Duration::from_secs(15))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(15)))?;
        stream.set_write_timeout(Some(Duration::from_secs(15)))?;
        match parsed.scheme {
            Scheme::Tcp => Ok(Connection::Tcp(stream)),
            Scheme::Ws => {
                let uri = format!("ws://{}", parsed.addr);
                let req = tungstenite::handshake::client::Request::builder()
                    .uri(&uri)
                    .header("Host", parsed.addr.to_string())
                    .header("Connection", "Upgrade")
                    .header("Upgrade", "websocket")
                    .header("Sec-WebSocket-Version", "13")
                    .header(
                        "Sec-WebSocket-Key",
                        tungstenite::handshake::client::generate_key(),
                    )
                    .body(())
                    .map_err(|e| TransportError::Handshake(e.to_string()))?;
                let (ws, _) = client_with_config(req, stream, None)
                    .map_err(|e| TransportError::Handshake(e.to_string()))?;
                Ok(Connection::Ws(ws))
            }
            Scheme::Unix => Err(TransportError::BadUrl(
                "unix connect is not a tcp address".into(),
            )),
        }
    }

    pub fn send_msg<T: Serialize>(&mut self, msg: &T) -> Result<(), TransportError> {
        let frame = protocol::encode_frame(msg)?;
        match self {
            Connection::Tcp(stream) => {
                use std::io::Write;
                stream.write_all(&frame)?;
                stream.flush()?;
            }
            Connection::Ws(ws) => {
                ws.send(Message::Binary(frame.into()))
                    .map_err(|e| ws_err(e))?;
            }
            #[cfg(unix)]
            Connection::Unix(stream) => {
                use std::io::Write;
                stream.write_all(&frame)?;
                stream.flush()?;
            }
        }
        Ok(())
    }

    pub fn recv_msg<T: DeserializeOwned>(&mut self) -> Result<T, TransportError> {
        match self {
            Connection::Tcp(stream) => Ok(protocol::read_frame(stream)?),
            #[cfg(unix)]
            Connection::Unix(stream) => Ok(protocol::read_frame(stream)?),
            Connection::Ws(ws) => loop {
                match ws.read() {
                    Ok(Message::Binary(data)) => {
                        return Ok(protocol::decode_frame(&data)?);
                    }
                    Ok(Message::Ping(p)) => {
                        let _ = ws.send(Message::Pong(p));
                    }
                    Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => {}
                    Ok(Message::Text(_)) => {
                        return Err(TransportError::Codec(CodecError::Postcard(
                            "expected binary websocket frame".into(),
                        )));
                    }
                    Ok(Message::Close(_)) => return Err(TransportError::Closed),
                    Err(tungstenite::Error::Io(e)) => return Err(TransportError::from(e)),
                    Err(e) => return Err(ws_err(e)),
                }
            },
        }
    }

    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), TransportError> {
        match self {
            Connection::Tcp(stream) => stream.set_read_timeout(timeout)?,
            #[cfg(unix)]
            Connection::Unix(stream) => stream.set_read_timeout(timeout)?,
            Connection::Ws(ws) => ws.get_mut().set_read_timeout(timeout)?,
        }
        Ok(())
    }

    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) -> Result<(), TransportError> {
        match self {
            Connection::Tcp(stream) => stream.set_write_timeout(timeout)?,
            #[cfg(unix)]
            Connection::Unix(stream) => stream.set_write_timeout(timeout)?,
            Connection::Ws(ws) => ws.get_mut().set_write_timeout(timeout)?,
        }
        Ok(())
    }

    pub fn peer_addr(&self) -> Option<SocketAddr> {
        match self {
            Connection::Tcp(s) => s.peer_addr().ok(),
            #[cfg(unix)]
            Connection::Unix(_) => None,
            Connection::Ws(ws) => ws.get_ref().peer_addr().ok(),
        }
    }

    pub fn close(self) -> Result<(), TransportError> {
        match self {
            Connection::Tcp(s) => {
                let _ = s.shutdown(Shutdown::Both);
            }
            #[cfg(unix)]
            Connection::Unix(s) => {
                let _ = s.shutdown(Shutdown::Both);
            }
            Connection::Ws(mut ws) => {
                let _ = ws.close(None);
                let _ = ws.get_mut().shutdown(Shutdown::Both);
            }
        }
        Ok(())
    }
}

fn connect_unix(url: &str) -> Result<Connection, TransportError> {
    let path = parse_unix_path(url)?;
    #[cfg(unix)]
    {
        let stream = std::os::unix::net::UnixStream::connect(&path)?;
        Connection::from_unix(stream)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(TransportError::BadUrl(
            "unix sockets are not supported on this OS".into(),
        ))
    }
}

fn ws_err(e: tungstenite::Error) -> TransportError {
    match e {
        tungstenite::Error::Io(io) => TransportError::from(io),
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
            TransportError::Closed
        }
        other => TransportError::Handshake(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PROTOCOL_VERSION;
    use crate::protocol::{ClientMessage, ServerMessage, hello};
    use std::thread;

    fn loopback(scheme: Scheme) {
        let prefix = match scheme {
            Scheme::Tcp => "tcp",
            Scheme::Ws => "ws",
            Scheme::Unix => "unix",
        };
        let listener = Listener::bind(&format!("{prefix}://127.0.0.1:0")).unwrap();
        let url = listener.local_url().unwrap();
        let server = thread::spawn(move || {
            let mut conn = listener.accept().unwrap();
            let hello: ClientMessage = conn.recv_msg().unwrap();
            match hello {
                ClientMessage::Hello {
                    protocol_version, ..
                } => assert_eq!(protocol_version, PROTOCOL_VERSION),
                other => panic!("expected Hello, got {other:?}"),
            }
            conn.send_msg(&ServerMessage::Welcome {
                tick: 0,
                state_hash: [0; 32],
                experiment_id: "test".into(),
            })
            .unwrap();
            conn.send_msg(&ServerMessage::Snapshot {
                checkpoint_bytes: b"AGTN".to_vec(),
            })
            .unwrap();
        });

        let mut client = Connection::connect(&url).unwrap();
        client.send_msg(&hello(None)).unwrap();
        let welcome: ServerMessage = client.recv_msg().unwrap();
        match welcome {
            ServerMessage::Welcome { experiment_id, .. } => {
                assert_eq!(experiment_id, "test");
            }
            other => panic!("expected Welcome, got {other:?}"),
        }
        let snap: ServerMessage = client.recv_msg().unwrap();
        match snap {
            ServerMessage::Snapshot { checkpoint_bytes } => {
                assert_eq!(checkpoint_bytes, b"AGTN");
            }
            other => panic!("expected Snapshot, got {other:?}"),
        }
        server.join().unwrap();
    }

    #[test]
    fn tcp_loopback_hello_snapshot() {
        loopback(Scheme::Tcp);
    }

    #[test]
    fn ws_loopback_hello_snapshot() {
        loopback(Scheme::Ws);
    }

    #[test]
    fn parse_unix_absolute_path() {
        let path = parse_unix_path("unix:///tmp/agentplace.sock").unwrap();
        assert_eq!(path, std::path::PathBuf::from("/tmp/agentplace.sock"));
        let err = parse_unix_path("unix://").unwrap_err();
        assert!(format!("{err}").contains("empty"), "{err}");
        let err = parse_unix_path("unix://@abstract").unwrap_err();
        assert!(format!("{err}").contains("abstract"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn unix_loopback_hello_snapshot() {
        let path = std::env::temp_dir().join(format!(
            "agentplace-m77-shared-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let url = format!("unix://{}", path.display());
        let listener = Listener::bind(&url).unwrap();
        let bound = listener.local_url().unwrap();
        let server = thread::spawn(move || {
            let mut conn = listener.accept().unwrap();
            let hello: ClientMessage = conn.recv_msg().unwrap();
            match hello {
                ClientMessage::Hello {
                    protocol_version, ..
                } => assert_eq!(protocol_version, PROTOCOL_VERSION),
                other => panic!("expected Hello, got {other:?}"),
            }
            conn.send_msg(&ServerMessage::Welcome {
                tick: 0,
                state_hash: [0; 32],
                experiment_id: "unix".into(),
            })
            .unwrap();
        });
        let mut client = Connection::connect(&bound).unwrap();
        client.send_msg(&hello(None)).unwrap();
        let welcome: ServerMessage = client.recv_msg().unwrap();
        match welcome {
            ServerMessage::Welcome { experiment_id, .. } => assert_eq!(experiment_id, "unix"),
            other => panic!("expected Welcome, got {other:?}"),
        }
        server.join().unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(not(unix))]
    #[test]
    fn unix_unsupported_on_this_os() {
        let err = Listener::bind("unix:///tmp/agentplace.sock").unwrap_err();
        assert!(format!("{err}").contains("not supported"), "{err}");
    }

    #[test]
    fn reject_wss() {
        let err = parse_listen_url("wss://127.0.0.1:9443").unwrap_err();
        assert!(format!("{err}").contains("wss"), "{err}");
    }
}
