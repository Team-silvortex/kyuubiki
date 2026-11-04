use std::cell::Cell;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub trait SocketIo: Read + Write {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()>;
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()>;
}

impl SocketIo for TcpStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        TcpStream::set_read_timeout(self, timeout)
    }
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        TcpStream::set_write_timeout(self, timeout)
    }
}

pub struct Connection {
    socket: TcpStream,
    tls: Option<Arc<Mutex<rustls::Connection>>>,
    read_deadline: Cell<Option<Instant>>,
    write_deadline: Cell<Option<Instant>>,
}

impl From<TcpStream> for Connection {
    fn from(socket: TcpStream) -> Self {
        Self {
            socket,
            tls: None,
            read_deadline: Cell::new(None),
            write_deadline: Cell::new(None),
        }
    }
}

impl Connection {
    pub(crate) fn handshake(
        socket: TcpStream,
        mut tls: rustls::Connection,
        timeout: Duration,
    ) -> io::Result<Self> {
        let mut connection = Self::from(socket);
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::other("invalid TLS deadline"))?;
        tls.set_buffer_limit(Some(64 * 1024));
        while tls.is_handshaking() {
            tls.complete_io(&mut TimedSocket {
                socket: &mut connection.socket,
                deadline: Some(deadline),
            })?;
        }
        if tls.alpn_protocol() != Some(b"kyuubiki-agent/1".as_slice()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Agent TLS protocol mismatch",
            ));
        }
        connection.tls = Some(Arc::new(Mutex::new(tls)));
        connection.read_deadline.set(Some(deadline));
        connection.write_deadline.set(Some(deadline));
        Ok(connection)
    }

    pub fn authenticated(&self) -> bool {
        self.tls.is_some()
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.socket.peer_addr()
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.socket.shutdown(how)
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            socket: self.socket.try_clone()?,
            tls: self.tls.clone(),
            read_deadline: self.read_deadline.clone(),
            write_deadline: self.write_deadline.clone(),
        })
    }
}

impl SocketIo for Connection {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        let deadline = deadline_for(timeout)?;
        self.socket.set_read_timeout(timeout)?;
        self.read_deadline.set(deadline);
        Ok(())
    }
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        let deadline = deadline_for(timeout)?;
        self.socket.set_write_timeout(timeout)?;
        self.write_deadline.set(deadline);
        Ok(())
    }
}

fn deadline_for(timeout: Option<Duration>) -> io::Result<Option<Instant>> {
    timeout
        .map(|value| {
            (!value.is_zero())
                .then(|| Instant::now().checked_add(value))
                .flatten()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid Agent timeout"))
        })
        .transpose()
}

impl Read for Connection {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let Some(tls) = &self.tls else {
            return self.socket.read(bytes);
        };
        let mut tls = tls
            .lock()
            .map_err(|_| io::Error::other("TLS state unavailable"))?;
        let mut socket = TimedSocket {
            socket: &mut self.socket,
            deadline: self.read_deadline.get(),
        };
        match &mut *tls {
            rustls::Connection::Client(tls) => rustls::Stream::new(tls, &mut socket).read(bytes),
            rustls::Connection::Server(tls) => rustls::Stream::new(tls, &mut socket).read(bytes),
        }
    }
}

impl Write for Connection {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(tls) = &self.tls else {
            return self.socket.write(bytes);
        };
        let mut tls = tls
            .lock()
            .map_err(|_| io::Error::other("TLS state unavailable"))?;
        let mut socket = TimedSocket {
            socket: &mut self.socket,
            deadline: self.write_deadline.get(),
        };
        // Do not acknowledge a buffered TLS write as a delivered Agent reply.
        let count = tls.writer().write(bytes)?;
        while tls.wants_write() {
            if tls.write_tls(&mut socket)? == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
        }
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}

struct TimedSocket<'a> {
    socket: &'a mut TcpStream,
    deadline: Option<Instant>,
}

impl TimedSocket<'_> {
    fn remaining(&self) -> io::Result<Option<Duration>> {
        self.deadline
            .map(|deadline| {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Agent transport deadline exceeded",
                    ))
                } else {
                    Ok(remaining)
                }
            })
            .transpose()
    }
}

impl Read for TimedSocket<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.socket.set_read_timeout(self.remaining()?)?;
        self.socket.read(bytes)
    }
}
impl Write for TimedSocket<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.socket.set_write_timeout(self.remaining()?)?;
        self.socket.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_timeout_keeps_existing_deadlines() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let stream = Connection::from(TcpStream::connect(listener.local_addr().unwrap()).unwrap());
        let _peer = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let read = stream.read_deadline.get();
        let write = stream.write_deadline.get();
        for invalid in [Duration::ZERO, Duration::MAX] {
            assert!(stream.set_read_timeout(Some(invalid)).is_err());
            assert!(stream.set_write_timeout(Some(invalid)).is_err());
            assert_eq!(stream.read_deadline.get(), read);
            assert_eq!(stream.write_deadline.get(), write);
        }
        stream.set_read_timeout(None).unwrap();
        stream.set_write_timeout(None).unwrap();
        assert_eq!(stream.read_deadline.get(), None);
        assert_eq!(stream.write_deadline.get(), None);
    }
}
