pub mod leaky_net_client;

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io;
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};

pub type TestNetClient = leaky_net_client::LeakyNetClient;

//consider channging buffer to a Cell or RefCell to allow internal mutation
pub struct NetClient {
    pub udp_socket: UdpSocket,
    pub buffer: [u8; 128],
    tcp_listener: TcpListener,
    tcp_stream: TcpStream,
}

impl NetClient {
    pub fn send<T: Serialize>(&self, data: &T) -> io::Result<usize> {
        let data = bincode::serialize(data).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "bincode serialization failed before sending a packet",
            )
        })?;
        self.udp_socket.send(&data)
    }
    pub fn recv<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        let len = self.udp_socket.recv(&mut self.buffer)?;
        let data = bincode::deserialize::<T>(&self.buffer[0..len]).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "bincode deserialization failed after recieving a packet",
            )
        })?;
        Ok(data)
    }

    pub fn write_tcp<T: Serialize>(&mut self, data: &T) -> io::Result<usize> {
        use std::io::Write;
        self.tcp_stream
            .write(&bincode::serialize(data).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bincode serialization failed after recieving a packet",
                )
            })?)
    }
    pub fn read_tcp<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        use std::io::Read;
        let size_read = self.tcp_stream.read(&mut self.buffer)?;
        bincode::deserialize::<T>(&self.buffer[..size_read]).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "bincode deserialization failed after recieving a packet",
            )
        })
    }

    pub fn connect<A: ToSocketAddrs + Copy + std::fmt::Debug>(target_addr: A) -> io::Result<Self> {
        let tcp_stream = TcpStream::connect(target_addr)?;
        let local_addr = tcp_stream.local_addr()?;
        let tcp_listener = TcpListener::bind(local_addr)?;
        let udp_socket = UdpSocket::bind(local_addr)?;
        udp_socket.connect(target_addr)?;
        udp_socket.set_nonblocking(true)?;
        Ok(NetClient {
            udp_socket,
            buffer: [0; 128],
            tcp_listener,
            tcp_stream,
        })
    }
    pub fn host<A: ToSocketAddrs + Copy + std::fmt::Debug>(local_addr: A) -> io::Result<Self> {
        let tcp_listener = TcpListener::bind(local_addr)?;
        let (tcp_stream, target_addr) = tcp_listener.accept()?;
        let local_addr = tcp_stream.local_addr()?;
        let udp_socket = UdpSocket::bind(local_addr)?;
        udp_socket.connect(target_addr)?;
        udp_socket.set_nonblocking(true)?;
        Ok(NetClient {
            udp_socket,
            buffer: [0; 128],
            tcp_listener,
            tcp_stream,
        })
    }
}
