use laminar::*;
use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Instant;

pub trait ClientNetSender {
    fn send_uu(&mut self, data: Box<[u8]>);
    fn send_ru(&mut self, data: Box<[u8]>);
    fn send_ro(&mut self, data: Box<[u8]>);
}

#[derive(Clone, Debug)]
pub enum NetEventKind {
    Data(Box<[u8]>),
    Connect,
    Disconnect,
}

#[derive(Clone, Debug)]
pub struct NetEvent {
    pub source: SocketAddr,
    pub kind: NetEventKind,
}

#[derive(Debug)]
pub struct ClientNetManager {
    sock: Socket,
    addr: SocketAddr,
}

impl ClientNetManager {
    pub fn new(addr: impl ToSocketAddrs) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        Self {
            sock: Socket::bind_any().unwrap(),
            addr,
        }
    }

    pub fn poll(&mut self) {
        self.sock.manual_poll(Instant::now());
    }

    pub fn recv(&mut self) -> impl Iterator<Item = NetEvent> + '_ {
        std::iter::from_fn(|| {
            self.sock.recv().map(|event| {
                let (source, kind) = match event {
                    SocketEvent::Packet(packet) => (
                        packet.addr(),
                        NetEventKind::Data(Box::from(packet.payload())),
                    ),
                    SocketEvent::Connect(addr) => (addr, NetEventKind::Connect),
                    SocketEvent::Disconnect(addr) => (addr, NetEventKind::Disconnect),
                    SocketEvent::Timeout(addr) => (addr, NetEventKind::Disconnect),
                };
                NetEvent { source, kind }
            })
        })
    }
}

impl ClientNetSender for ClientNetManager {
    fn send_uu(&mut self, data: Box<[u8]>) {
        let packet = Packet::unreliable_sequenced(self.addr, Vec::from(data), None);
        self.sock.send(packet).unwrap();
    }

    fn send_ru(&mut self, data: Box<[u8]>) {
        let packet = Packet::reliable_unordered(self.addr, Vec::from(data));
        self.sock.send(packet).unwrap();
    }

    fn send_ro(&mut self, data: Box<[u8]>) {
        let packet = Packet::reliable_ordered(self.addr, Vec::from(data), None);
        self.sock.send(packet).unwrap();
    }
}

#[derive(Debug)]
pub struct ServerNetManager {
    sock: Socket,
}

impl ServerNetManager {
    pub fn new(bind_port: u16) -> (Self, u16) {
        let sock = Socket::bind(("0.0.0.0", bind_port)).unwrap();
        let bind_port = sock.local_addr().unwrap().port();
        (Self { sock }, bind_port)
    }

    pub fn poll(&mut self) {
        self.sock.manual_poll(Instant::now());
    }

    pub fn recv(&mut self) -> impl Iterator<Item = NetEvent> + '_ {
        std::iter::from_fn(|| {
            self.sock.recv().map(|event| {
                let (source, kind) = match event {
                    SocketEvent::Packet(packet) => (
                        packet.addr(),
                        NetEventKind::Data(Box::from(packet.payload())),
                    ),
                    SocketEvent::Connect(addr) => (addr, NetEventKind::Connect),
                    SocketEvent::Disconnect(addr) => (addr, NetEventKind::Disconnect),
                    SocketEvent::Timeout(addr) => (addr, NetEventKind::Disconnect),
                };
                NetEvent { source, kind }
            })
        })
    }

    pub fn send_uu(&mut self, dst: impl ToSocketAddrs, data: Box<[u8]>) {
        let dst = dst.to_socket_addrs().unwrap().next().unwrap();
        let packet = Packet::unreliable_sequenced(dst.into(), Vec::from(data), None);
        self.sock.send(packet).unwrap();
    }

    pub fn send_ru(&mut self, dst: impl ToSocketAddrs, data: Box<[u8]>) {
        let dst = dst.to_socket_addrs().unwrap().next().unwrap();
        let packet = Packet::reliable_unordered(dst.into(), Vec::from(data));
        self.sock.send(packet).unwrap();
    }

    pub fn send_ro(&mut self, dst: impl ToSocketAddrs, data: Box<[u8]>) {
        let dst = dst.to_socket_addrs().unwrap().next().unwrap();
        let packet = Packet::reliable_ordered(dst.into(), Vec::from(data), None);
        self.sock.send(packet).unwrap();
    }
}
