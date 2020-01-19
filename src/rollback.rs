use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{ToSocketAddrs, UdpSocket};

pub struct RollbackClient {
    socket: UdpSocket,
    buffer: [u8; 128],
    connected: bool,
}

#[derive(Serialize, Deserialize, Debug)]
enum RollbackPacket<T> {
    Hello,
    Goodbye,
    Info(T),
}

impl RollbackClient {
    pub fn new<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let mut socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true);
        Ok(Self {
            socket,
            buffer: [0; 128],
            connected: false,
        })
    }

    pub fn connect<A: ToSocketAddrs>(addr: A) {}

    pub fn block_for_connection() {}

    /// returns whether or not you need to check the socket again
    pub fn check_socket<T: Serialize + DeserializeOwned>(
        &mut self,
    ) -> io::Result<(bool, Option<T>)> {
        match self.socket.recv_from(&mut self.buffer) {
            Ok((len, src)) => {
                let data: RollbackPacket<T> = match bincode::deserialize(&self.buffer[..len]) {
                    Ok(data) => data,
                    Err(e) => {
                        println!("invalid packet read: {:?}", e);
                        return Ok((true, None));
                    }
                };
                match data {
                    RollbackPacket::Hello => {
                        self.socket.connect(&src)?;
                        self.connected = true;
                        Ok((true, None))
                    }
                    RollbackPacket::Info(data) => Ok((true, Some(data))),
                    RollbackPacket::Goodbye => {
                        self.connected = false;
                        Ok((true, None))
                    }
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok((false, None)),
            Err(e) => Err(e),
        }
    }
}
