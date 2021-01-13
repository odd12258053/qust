use crate::command::Command;
use crate::message::{Reply, Request, TERMINATION};
use crate::queue::QueueManager;
use crate::signal::Sig;
use crate::utils::is_delimiter;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Registry, Token, Waker};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const SERVER: Token = Token(0);
const WAKER: Token = Token(1);
const START_POINT: usize = 2;
const BUFFER_SIZE: usize = 128 * 1024; // 128KB
const CONN_SIZE: usize = 128;
const EVENTS_SIZE: usize = 1 * 1024;
const MAX_BUFFER_SIZE: usize = 1 * 1024 * 1024 + 1024; // about 1MB

#[inline]
fn next(current: &mut Token) -> Token {
    let next = current.0;
    match current.0.checked_add(1) {
        Some(v) => current.0 = v,
        None => current.0 = START_POINT,
    }
    Token(next)
}

#[inline]
fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

#[inline]
fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}

struct Connection {
    conn: TcpStream,
    reply: Vec<u8>,
    received_data: Vec<u8>,
}

impl Connection {
    fn new(conn: TcpStream) -> Connection {
        Connection {
            conn,
            reply: vec![0; 0],
            received_data: vec![0; 0],
        }
    }
    fn clean(&mut self) {
        self.reply = vec![0; 0];
        self.received_data = vec![0; 0];
    }
}

pub struct Server {
    addr: SocketAddr,
    token: Token,
    connections: HashMap<Token, Connection>,
    buffer: [u8; BUFFER_SIZE],
}

impl Server {
    pub fn new(addr: SocketAddr) -> Server {
        Server {
            addr,
            token: Token(START_POINT),
            connections: HashMap::with_capacity(CONN_SIZE),
            buffer: [0; BUFFER_SIZE],
        }
    }

    #[inline]
    fn serve(&mut self, registry: &Registry, server: &TcpListener) -> io::Result<()> {
        loop {
            // Received an event for the TCP server socket, which
            // indicates we can accept an connection.
            match server.accept() {
                Ok((mut connection, address)) => {
                    debug!("Accepted connection from: {}", address);
                    let token = next(&mut self.token);
                    registry.register(&mut connection, token, Interest::READABLE)?;
                    self.connections.insert(token, Connection::new(connection));
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // If we get a `WouldBlock` error we know our
                    // listener has no more incoming connections queued,
                    // so we can return to polling and wait for some
                    // more.
                    return Ok(());
                }
                Err(e) => {
                    // If it was any other kind of error, something went
                    // wrong and we terminate with an error.
                    return Err(e);
                }
            };
        }
    }
    #[inline]
    fn wake(&mut self, registry: &Registry, receiver: &Receiver<Box<Reply>>) -> io::Result<()> {
        loop {
            match receiver.recv_timeout(Duration::from_nanos(1)) {
                Ok(rep) => {
                    debug!(
                        "Catch reply: {:?} {:?} {:?} [{:p}]",
                        rep.token,
                        rep.status,
                        rep.data.len(),
                        rep
                    );
                    let token = rep.token;
                    match self.connections.get_mut(&token) {
                        Some(connection) => {
                            connection.reply = rep.message();
                            registry.reregister(&mut connection.conn, token, Interest::WRITABLE)?;
                        }
                        None => {}
                    };
                }
                Err(_) => return Ok(()),
            };
        }
    }
    #[inline]
    fn handle_to_write(&mut self, registry: &Registry, token: Token) -> io::Result<()> {
        let connection = self.connections.get_mut(&token).unwrap();
        debug!("Reply size: {}", connection.reply.len());
        // We can (maybe) write to the connection.

        match connection.conn.write(connection.reply.as_slice()) {
            // We want to write the entire `DATA` buffer in a single go. If we
            // write less we'll return a short write error (same as
            // `io::Write::write_all` does).
            Ok(n) if n < connection.reply.len() => {
                connection.reply = connection.reply.drain(n..).collect();
                debug!("Catch Error; n<len");
                registry.reregister(&mut connection.conn, token, Interest::WRITABLE)?;
                // return Err(io::ErrorKind::WriteZero.into())
            }
            Ok(_) => {
                // After we've written something we'll re-register the connection
                // to only respond to readable events.
                connection.clean();
                registry.reregister(&mut connection.conn, token, Interest::READABLE)?;
            }
            // Would block "errors" are the OS's way of saying that the
            // connection is not actually ready to perform this I/O operation.
            Err(ref err) if would_block(err) => {
                debug!("has would_block");
                registry.reregister(&mut connection.conn, token, Interest::WRITABLE)?;
            }
            // Got interrupted (how rude!), we'll try again.
            Err(ref err) if interrupted(err) => {
                debug!("has would_block");
                registry.reregister(&mut connection.conn, token, Interest::WRITABLE)?;
            }
            // Other errors we'll consider fatal.
            Err(err) => {
                // return Err(err)
                debug!("error: {}", err);
            }
        }
        Ok(())
    }
    #[inline]
    fn handle_to_read(
        &mut self,
        registry: &Registry,
        token: Token,
        sender: &Sender<Box<Request>>,
    ) -> io::Result<()> {
        let connection = match self.connections.get_mut(&token) {
            Some(c) => c,
            None => return Ok(()),
        };
        // We can (maybe) read from the connection.
        match connection.conn.read(&mut self.buffer) {
            Ok(0) => {
                // Reading 0 bytes means the other side has closed the
                // connection or is done writing, then so are we.
                debug!(
                    "Closed connection from: {}",
                    connection.conn.local_addr().unwrap()
                );
                self.connections.remove(&token);
                return Ok(());
            }
            Ok(n) => {
                if n + connection.received_data.len() > MAX_BUFFER_SIZE {
                    connection.clean();
                    connection.reply = Reply::error(token).message();
                    registry.reregister(&mut connection.conn, token, Interest::WRITABLE)?;
                    return Ok(());
                }
                connection.received_data.extend(&self.buffer[0..n]);
                if connection.received_data[connection.received_data.len() - 1] != TERMINATION {
                    registry.reregister(&mut connection.conn, token, Interest::READABLE)?;
                    return Ok(());
                }
            }
            // Would block "errors" are the OS's way of saying that the
            // connection is not actually ready to perform this I/O operation.
            Err(ref err) if would_block(err) => {
                debug!("has would_block");
                registry.reregister(&mut connection.conn, token, Interest::READABLE)?;
            }
            Err(ref err) if interrupted(err) => {
                debug!("has interrupted");
                registry.reregister(&mut connection.conn, token, Interest::READABLE)?;
            }
            // Other errors we'll consider fatal.
            Err(err) => {
                error!("{}", err);
                debug!(
                    "Closed connection from: {}",
                    connection.conn.local_addr().unwrap()
                );
                let _ = connection.conn.shutdown(Shutdown::Both);
                self.connections.remove(&token);
                return Ok(());
            }
        }
        debug!("received_data: {}", connection.received_data.len());
        // `\n`を除く
        let mut iter =
            connection.received_data[..connection.received_data.len() - 1].splitn(2, is_delimiter);
        match Command::from(iter.next().unwrap()) {
            Some(Command::QUIT) => {
                connection.conn.shutdown(Shutdown::Both)?;
                debug!(
                    "Closed connection from: {}",
                    connection.conn.local_addr().unwrap()
                );
                self.connections.remove(&token);
                return Ok(());
            }
            Some(cmd) => {
                let arg = match iter.next() {
                    Some(a) => a.to_vec(),
                    None => vec![0; 0],
                };
                let req = Box::new(Request { token, cmd, arg });
                debug!(
                    "Send Request: {:?} {:?} {:?} [{:p}]",
                    req.token,
                    req.cmd,
                    req.arg.len(),
                    req
                );
                sender.send(req).unwrap();
                connection.clean();
            }
            None => {
                if connection.received_data.len() == 1 {
                    connection.clean();
                    registry.reregister(&mut connection.conn, token, Interest::READABLE)?;
                } else {
                    connection.clean();
                    connection.reply = Reply::error(token).message();
                    registry.reregister(&mut connection.conn, token, Interest::WRITABLE)?;
                }
                return Ok(());
            }
        }
        Ok(())
    }
    pub fn run(addr: SocketAddr) -> io::Result<()> {
        let mut app = Server::new(addr);
        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(EVENTS_SIZE);
        let mut server = TcpListener::bind(app.addr)?;

        poll.registry()
            .register(&mut server, SERVER, Interest::READABLE)?;

        let (req_tx, req_rx) = channel::<Box<Request>>();
        let (rep_tx, rep_rx) = channel::<Box<Reply>>();

        let waker = Arc::new(Waker::new(poll.registry(), WAKER)?);
        let queue = QueueManager::run(waker.clone(), rep_tx, req_rx);
        let stat = Arc::new(AtomicBool::new(false));
        let sig = Sig::new(stat.clone());

        let job = thread::spawn(move || loop {
            poll.poll(&mut events, None).unwrap();
            let registry = poll.registry();
            for event in events.iter() {
                match event.token() {
                    SERVER => app.serve(registry, &server).unwrap(),
                    WAKER => {
                        if Arc::try_unwrap(Arc::clone(&stat))
                            .unwrap_err()
                            .load(Ordering::Relaxed)
                        {
                            info!("Shutdown");
                            req_tx
                                .send(Box::new(Request {
                                    token: WAKER,
                                    cmd: Command::TERMINATE,
                                    arg: vec![0; 0],
                                }))
                                .unwrap();
                            return;
                        }
                        app.wake(registry, &rep_rx).unwrap();
                    }
                    token => {
                        if event.is_writable() {
                            app.handle_to_write(registry, token).unwrap();
                        } else if event.is_readable() {
                            app.handle_to_read(registry, token, &req_tx).unwrap();
                        }
                    }
                }
            }
        });
        sig.run(waker.clone());
        queue.join().unwrap();
        job.join().unwrap();
        Ok(())
    }
}
