use super::NetClient;
use rand::random;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io;
use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};

pub struct LeakyNetClient {
    internal_client: NetClient,
    delayed_packets: Vec<(Vec<u8>, Instant)>,
    pub packet_loss: f32,
    pub delay: Duration,
}

impl LeakyNetClient {
    fn new(internal_client: NetClient) -> Self {
        LeakyNetClient {
            internal_client,
            delayed_packets: Vec::new(),
            packet_loss: 0.0,
            delay: Duration::from_millis(0),
        }
    }

    fn handle_packet<T: Serialize>(&mut self, data: &T) -> io::Result<()> {
        if 1.0 - self.packet_loss > random() {
            let raw_data = bincode::serialize(data).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "bincode serialization failed before sending a packet",
                )
            })?;
            self.delayed_packets.push((raw_data, Instant::now()));
        }
        Ok(())
    }

    pub fn send<T: Serialize + std::fmt::Debug>(&mut self, data: &T) -> io::Result<usize> {
        self.handle_packet(data)?;
        let res = Ok(self
            .delayed_packets
            .last()
            .map(|item| item.0.len())
            .unwrap_or(0));
        self.send_queued()?;
        res
    }

    pub fn send_queued(&mut self) -> io::Result<()> {
        let current_time_adjusted = Instant::now() - self.delay;
        let no_delay = self.delay == Duration::from_millis(0);
        for (data, _) in self
            .delayed_packets
            .iter()
            .filter(|(_, time)| *time < current_time_adjusted || no_delay)
        {
            self.internal_client.udp_socket.send(data)?;
        }
        self.delayed_packets
            .retain(|(_, time)| *time >= current_time_adjusted && !no_delay);
        Ok(())
    }

    pub fn recv<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        self.internal_client.recv()
    }

    pub fn write_tcp<T: Serialize>(&mut self, data: &T) -> io::Result<usize> {
        self.internal_client.write_tcp(data)
    }
    pub fn read_tcp<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        self.internal_client.read_tcp()
    }

    pub fn connect<A: ToSocketAddrs + Copy + std::fmt::Debug>(addr: A) -> io::Result<Self> {
        Ok(Self::new(NetClient::connect(addr)?))
    }

    pub fn host<A: ToSocketAddrs + Copy + std::fmt::Debug>(addr: A) -> io::Result<Self> {
        Ok(Self::new(NetClient::host(addr)?))
    }
}
