use crossbeam_channel::{Receiver, Sender};
use laminar::*;

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Instant;

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
    send: Sender<Packet>,
    recv: Receiver<SocketEvent>,
}

#[allow(dead_code)]
impl ClientNetManager {
    pub fn new(addr: impl ToSocketAddrs) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        let sock = Socket::bind_any().unwrap();
        let send = sock.get_packet_sender();
        let recv = sock.get_event_receiver();
        Self {
            sock,
            addr,
            send,
            recv,
        }
    }

    pub fn poll(&mut self) {
        self.sock.manual_poll(Instant::now());
    }

    pub fn recv(&self) -> impl Iterator<Item = NetEvent> + '_ {
        self.recv.try_iter().map(|event| {
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
    }

    pub fn send_uu(&self, data: Box<[u8]>) {
        let packet = Packet::unreliable_sequenced(self.addr, Vec::from(data), None);
        self.send.send(packet).unwrap();
    }

    pub fn send_ru(&self, data: Box<[u8]>) {
        let packet = Packet::reliable_unordered(self.addr, Vec::from(data));
        self.send.send(packet).unwrap();
    }

    pub fn send_ro(&self, data: Box<[u8]>) {
        let packet = Packet::reliable_ordered(self.addr, Vec::from(data), None);
        self.send.send(packet).unwrap();
    }
}

#[derive(Debug)]
pub struct ServerNetManager {
    sock: Socket,
    send: Sender<Packet>,
    recv: Receiver<SocketEvent>,
}

#[allow(dead_code)]
impl ServerNetManager {
    pub fn new(bind_port: u16) -> (Self, u16) {
        let sock = Socket::bind(("0.0.0.0", bind_port)).unwrap();
        let bind_port = sock.local_addr().unwrap().port();
        let send = sock.get_packet_sender();
        let recv = sock.get_event_receiver();
        (Self { sock, send, recv }, bind_port)
    }

    pub fn poll(&mut self) {
        self.sock.manual_poll(Instant::now());
    }

    pub fn recv(&self) -> impl Iterator<Item = NetEvent> + '_ {
        self.recv.try_iter().map(|event| {
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
    }

    pub fn send_uu(&self, dst: impl ToSocketAddrs, data: Box<[u8]>) {
        let dst = dst.to_socket_addrs().unwrap().next().unwrap();
        let packet = Packet::unreliable_sequenced(dst.into(), Vec::from(data), None);
        self.send.send(packet).unwrap();
    }

    pub fn send_ru(&self, dst: impl ToSocketAddrs, data: Box<[u8]>) {
        let dst = dst.to_socket_addrs().unwrap().next().unwrap();
        let packet = Packet::reliable_unordered(dst.into(), Vec::from(data));
        self.send.send(packet).unwrap();
    }

    pub fn send_ro(&self, dst: impl ToSocketAddrs, data: Box<[u8]>) {
        let dst = dst.to_socket_addrs().unwrap().next().unwrap();
        let packet = Packet::reliable_ordered(dst.into(), Vec::from(data), None);
        self.send.send(packet).unwrap();
    }
}
