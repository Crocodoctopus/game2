use crate::net::{NetEventKind, ServerNetManager};
use crate::server::log;
use crate::shared::*;
use std::path::Path;

pub struct GameUpdateState {
    // Net manager.
    net_manager: ServerNetManager,

    // Tiles.
    world_w: usize,
    world_h: usize,
    chunk_seqs: Box<[u32]>,
    fg_tiles: Box<[Tile]>,
    bg_tiles: Box<[Tile]>,
}

impl GameUpdateState {
    pub fn new(_root: &'static Path, net_manager: ServerNetManager) -> Self {
        let world_w = 8400;
        let world_h = 2400;
        let chunk_seqs = vec![1; world_w * world_h].into_boxed_slice();
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

                if y < 102 {
                    fg_tiles[index] = Tile::None;
                    bg_tiles[index] = Tile::None;
                    continue;
                }

                if y < 102 + 5 {
                    fg_tiles[index] = Tile::Dirt;
                    bg_tiles[index] = Tile::Dirt;
                    continue;
                }

                if y < 102 + 15 {
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
            chunk_seqs,
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
        for e in self.net_manager.recv() {
            let source = e.source;
            match e.kind {
                NetEventKind::Data(bytes) => {
                    for msg in deserialize(bytes).into_vec() {
                        match msg {
                            ClientNetMessage::Connect { .. } => {
                                // Accept all connections.
                                self.net_manager
                                    .send_ru(source, serialize(&[ServerNetMessage::ConnectAccept]));
                                // TODO push to connection hashmap
                            }

                            ClientNetMessage::RequestChunk { x, y, seq } => {
                                // TODO seq
                                let cx = x as usize;
                                let cy = y as usize;
                                let cur_seq =
                                    &mut self.chunk_seqs[cx + cy * self.world_w / CHUNK_SIZE];

                                if *cur_seq <= seq {
                                    continue;
                                }

                                *cur_seq = seq;
                                let mut fg_tiles = [Tile::None; CHUNK_AREA];
                                let mut bg_tiles = [Tile::None; CHUNK_AREA];
                                for y in 0..CHUNK_SIZE {
                                    for x in 0..CHUNK_SIZE {
                                        let src_index = x
                                            + cx * CHUNK_SIZE
                                            + (y + cy * CHUNK_SIZE) * self.world_w;
                                        let dst_index = x + y * CHUNK_SIZE;
                                        fg_tiles[dst_index] = self.fg_tiles[src_index];
                                        bg_tiles[dst_index] = self.bg_tiles[src_index];
                                    }
                                }

                                self.net_manager.send_ru(
                                    source,
                                    serialize(&[ServerNetMessage::ChunkSync {
                                        x,
                                        y,
                                        seq: seq + 1,
                                        fg_tiles,
                                        bg_tiles,
                                    }]),
                                );
                                // Don't flood the console with join ChunkRequests.
                                log!("Chunk ({}, {}) sent to {:?}.", cx, cy, source);
                            }

                            ClientNetMessage::Join => {
                                let mut msgs = Vec::new();

                                msgs.push(ServerNetMessage::WorldInfo {
                                    width: self.world_w as u16,
                                    height: self.world_h as u16,
                                    spawn_x: 100,
                                    spawn_y: 100,
                                });

                                // Arbitrary spawn point.
                                let spawn_x = 100 * TILE_SIZE;
                                let spawn_y = 100 * TILE_SIZE;
                                let viewport_w = 1920;
                                let viewport_h = 1080;

                                // Calculate load area.
                                const TILE_CHUNK_SIZE: usize = TILE_SIZE * CHUNK_SIZE;
                                let x1 = (spawn_x - viewport_w / 2) / TILE_CHUNK_SIZE;
                                let x2 = (spawn_x + viewport_w / 2 + TILE_CHUNK_SIZE - 1)
                                    / TILE_CHUNK_SIZE;
                                let y1 = (spawn_y - viewport_h / 2) / TILE_CHUNK_SIZE;
                                let y2 = (spawn_y + viewport_h / 2 + TILE_CHUNK_SIZE - 1)
                                    / TILE_CHUNK_SIZE;

                                // Send chunk data.
                                for cy in y1..y2 {
                                    for cx in x1..x2 {
                                        let mut fg_tiles = [Tile::None; CHUNK_AREA];
                                        let mut bg_tiles = [Tile::None; CHUNK_AREA];
                                        for y in 0..CHUNK_SIZE {
                                            for x in 0..CHUNK_SIZE {
                                                let src_index = x
                                                    + cx * CHUNK_SIZE
                                                    + (y + cy * CHUNK_SIZE) * self.world_w;
                                                let dst_index = x + y * CHUNK_SIZE;
                                                fg_tiles[dst_index] = self.fg_tiles[src_index];
                                                bg_tiles[dst_index] = self.bg_tiles[src_index];
                                            }
                                        }

                                        msgs.push(ServerNetMessage::ChunkSync {
                                            x: cx as u16,
                                            y: cy as u16,
                                            seq: 1,
                                            fg_tiles,
                                            bg_tiles,
                                        });
                                    }
                                }

                                // Send end.
                                msgs.push(ServerNetMessage::Start);

                                let se = serialize(&msgs);
                                log!("Initial sync with size {}.", se.len());

                                self.net_manager.send_ru(source, se);
                                self.net_manager
                                    .send_ru(source, serialize(&[ServerNetMessage::Ping]));
                            }

                            ClientNetMessage::Ping => self
                                .net_manager
                                .send_ru(source, serialize(&[ServerNetMessage::Ping])),
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
