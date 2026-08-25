//! Blocking TCP and WebSocket transports. Same postcard codec on both.

use crate::protocol::{self, CodecError};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{self, ErrorKind};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;
use tungstenite::protocol::WebSocket;
use tungstenite::{accept as ws_accept, client::client_with_config, Message};

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListenUrl {
    pub scheme: Scheme,
    pub addr: SocketAddr,
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

pub struct Listener {
    inner: TcpListener,
    scheme: Scheme,
}

impl Listener {
    pub fn bind(url: &str) -> Result<Self, TransportError> {
        let parsed = parse_listen_url(url)?;
        let inner = TcpListener::bind(parsed.addr)?;
        inner.set_nonblocking(false)?;
        Ok(Self {
            inner,
            scheme: parsed.scheme,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        Ok(self.inner.local_addr()?)
    }

    pub fn local_url(&self) -> Result<String, TransportError> {
        let addr = self.local_addr()?;
        let scheme = match self.scheme {
            Scheme::Tcp => "tcp",
            Scheme::Ws => "ws",
        };
        Ok(format!("{scheme}://{addr}"))
    }

    pub fn accept(&self) -> Result<Connection, TransportError> {
        let (stream, _) = self.inner.accept()?;
        self.finish_accept(stream)
    }

    /// Returns `Ok(None)` when no connection is pending.
    pub fn accept_nonblocking(&self) -> Result<Option<Connection>, TransportError> {
        self.inner.set_nonblocking(true)?;
        let result = self.inner.accept();
        self.inner.set_nonblocking(false)?;
        match result {
            Ok((stream, _)) => Ok(Some(self.finish_accept(stream)?)),
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn finish_accept(&self, stream: TcpStream) -> Result<Connection, TransportError> {
        stream.set_nodelay(true)?;
        stream.set_nonblocking(false)?;
        match self.scheme {
            Scheme::Tcp => Ok(Connection::Tcp(stream)),
            Scheme::Ws => {
                let ws = ws_accept(stream).map_err(|e| TransportError::Handshake(e.to_string()))?;
                Ok(Connection::Ws(ws))
            }
        }
    }
}

pub enum Connection {
    Tcp(TcpStream),
    Ws(WebSocket<TcpStream>),
}

impl Connection {
    pub fn connect(url: &str) -> Result<Self, TransportError> {
        let parsed = parse_listen_url(url)?;
        let stream = TcpStream::connect(parsed.addr)?;
        stream.set_nodelay(true)?;
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
        }
        Ok(())
    }

    pub fn recv_msg<T: DeserializeOwned>(&mut self) -> Result<T, TransportError> {
        match self {
            Connection::Tcp(stream) => Ok(protocol::read_frame(stream)?),
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
            Connection::Ws(ws) => ws.get_mut().set_read_timeout(timeout)?,
        }
        Ok(())
    }

    pub fn peer_addr(&self) -> Option<SocketAddr> {
        match self {
            Connection::Tcp(s) => s.peer_addr().ok(),
            Connection::Ws(ws) => ws.get_ref().peer_addr().ok(),
        }
    }

    pub fn close(self) -> Result<(), TransportError> {
        match self {
            Connection::Tcp(s) => {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
            Connection::Ws(mut ws) => {
                let _ = ws.close(None);
            }
        }
        Ok(())
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
    use crate::protocol::{hello, ClientMessage, ServerMessage};
    use crate::PROTOCOL_VERSION;
    use std::thread;

    fn loopback(scheme: Scheme) {
        let prefix = match scheme {
            Scheme::Tcp => "tcp",
            Scheme::Ws => "ws",
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
    fn reject_wss() {
        let err = parse_listen_url("wss://127.0.0.1:9443").unwrap_err();
        assert!(format!("{err}").contains("wss"), "{err}");
    }
}
