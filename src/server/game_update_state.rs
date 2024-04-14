use crate::net::{NetEvent, NetEventKind, ServerNetManager};
use crate::shared::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

pub struct GameUpdateState {
    net_manager: ServerNetManager,

    // Tiles.
    world_w: usize,
    world_h: usize,
    //chunk_seqs: Box<[u32]>,
    fg_tiles: Box<[Tile]>,
    bg_tiles: Box<[Tile]>,
}

impl GameUpdateState {
    pub fn new(root: &'static Path, net_manager: ServerNetManager) -> Self {
        let world_w = 8400;
        let world_h = 2400;
        let mut fg_tiles = vec![Tile::None; world_w * world_h].into_boxed_slice();
        let mut bg_tiles = vec![Tile::None; world_w * world_h].into_boxed_slice();
        for y in 0..world_h {
            for x in 0..world_w {
                let index = x + y * world_w;

                if y == 0 || y == world_h - 1 || x == 0 || x == world_w - 1 {
                    fg_tiles[index] = Tile::Dirt;
                    bg_tiles[index] = Tile::Dirt;
                    continue;
                }

                if y < 20 {
                    fg_tiles[index] = Tile::None;
                    bg_tiles[index] = Tile::None;
                    continue;
                }

                if y < 25 {
                    fg_tiles[index] = Tile::Dirt;
                    bg_tiles[index] = Tile::Dirt;
                    continue;
                }

                if y < 45 {
                    fg_tiles[index] = Tile::Stone;
                    bg_tiles[index] = Tile::Stone;
                    continue;
                }

                fg_tiles[index] = Tile::DenseStone;
                bg_tiles[index] = Tile::DenseStone;
            }
        }

        Self {
            net_manager,

            world_w,
            world_h,
            //chunk_seqs: vec![1; world_w * world_h / CHUNK_AREA],
            fg_tiles,
            bg_tiles,
        }
    }

    pub fn prestep(&mut self, ts: u64) {
        // Poll for event receiving.
        self.net_manager.poll();
        self.handle_net_events(ts);
    }

    pub fn step(&mut self, _ts: u64, _ft: u64) {}

    pub fn poststep(&mut self, _ts: u64) {
        // Poll for event sending.
        self.net_manager.poll();
    }

    fn handle_net_events(&mut self, _ts: u64) {
        let tmp: Vec<NetEvent> = self.net_manager.recv().collect();
        for e in tmp {
            match e.kind {
                NetEventKind::Data(bytes) => {
                    let msgs: Vec<ClientNetMessage> = deserialize(bytes).into_vec();
                    println!("[Server] {msgs:?}");
                    for msg in msgs {
                        match msg {
                            ClientNetMessage::Connect { .. } => {
                                // Accept all connections.
                                self.net_manager.send_ru(
                                    e.source,
                                    serialize(&[ServerNetMessage::ConnectAccept]),
                                );
                                // TODO push to connection hashmap
                            }

                            ClientNetMessage::RequestChunk { x, y, seq } => {
                                /*
                                let mut fg_tiles = Vec::with_capacity(CHUNK_AREA);
                                let mut bg_tiles = Vec::with_capacity(CHUNK_AREA);
                                self.net_manager.send_ru(
                                    e.source,
                                    serialize(&[ServerNetMessage::ChunkSync {
                                        x,
                                        y,

                                    }]);
                                );
                                */
                            }

                            ClientNetMessage::Join => {
                                self.net_manager.send_ro(
                                    e.source,
                                    serialize(&[ServerNetMessage::WorldInfo {
                                        width: self.world_w as u16,
                                        height: self.world_h as u16,
                                    }]),
                                );
                                self.net_manager.send_ro(
                                    e.source,
                                    serialize(&[ServerNetMessage::ChunkSync {
                                        x: 0,
                                        y: 0,
                                        seq: 0,
                                        fg_tiles: [Tile::Dirt; CHUNK_AREA],
                                        bg_tiles: [Tile::Dirt; CHUNK_AREA],
                                    }]),
                                );

                                self.net_manager
                                    .send_ro(e.source, serialize(&[ServerNetMessage::Start]));
                                /*
                                     * let tmp = serialize(&[ServerNetMessage::WorldStateSync {
                                        width: self.world_w as u16,
                                        height: self.world_h as u16,
                                        fg_tiles: self.fg_tiles.clone(),
                                        bg_tiles: self.bg_tiles.clone(),
                                    }]);
                                    println!("{}", tmp.len());
                                    self.net_manager.send_ru(e.source, tmp);
                                */
                            }
                        }
                    }
                }

                NetEventKind::Disconnect => {}

                // Ignore internal connections.
                NetEventKind::Connect => {}
            }
        }
    }
}
