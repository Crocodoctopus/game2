use crate::net::{NetEventKind, ServerNetManager};
use crate::server::log;
use crate::shared::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

pub struct Connection {
    // Whether the client has joined yet.
    joined: bool,

    // Disconnect flag.
    disconnect: bool,

    // The ID this connection owns.
    id: Option<HumanoidId>,
}

pub struct GameUpdateState {
    // Net manager.
    net_manager: ServerNetManager,
    connections: HashMap<SocketAddr, Connection>,

    // Tiles.
    world_w: usize,
    world_h: usize,
    chunk_seqs: Box<[u32]>,
    fg_tiles: Box<[Tile]>,
    bg_tiles: Box<[Tile]>,

    // Players.
    humanoid_id_counter: HumanoidId,
    humanoids: HashMap<HumanoidId, Humanoid>,
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
        fg_tiles[108 + 102 * world_w] = Tile::None;
        fg_tiles[109 + 102 * world_w] = Tile::None;
        fg_tiles[107 + 102 * world_w] = Tile::None;
        fg_tiles[106 + 102 * world_w] = Tile::None;
        fg_tiles[107 + 103 * world_w] = Tile::None;
        fg_tiles[106 + 103 * world_w] = Tile::None;

        let mut humanoid_id_counter = HumanoidId::new();
        let mut humanoids = HashMap::new();
        let spawn_x = 100 * TILE_SIZE;
        let spawn_y = 100 * TILE_SIZE - 32;

        humanoids.insert(
            humanoid_id_counter.next(),
            Humanoid {
                base: HumanoidBase {
                    x: spawn_x as f32 + 256.,
                    y: spawn_y as f32,
                    w: 32. - 8.,
                    h: 48. - 8.,
                    flags: 0,
                },
                ai: HumanoidAi::Zombie,
                input: HumanoidInput::default(),
                physics: HumanoidPhysics {
                    max_dx: 50.,
                    ..Default::default()
                },
            },
        );

        Self {
            net_manager,
            connections: HashMap::new(),

            world_w,
            world_h,
            chunk_seqs,
            fg_tiles,
            bg_tiles,

            humanoid_id_counter,
            humanoids,
        }
    }

    pub fn prestep(&mut self, ts: u64) {
        // Poll for event receiving.
        self.net_manager.poll();
        self.handle_net_events(ts);
    }

    pub fn step(&mut self, _ts: u64, ft: u64) {
        let ft = ft as f32 / 1e6;

        // Humanoid AI pass.
        update_humanoid_ais(&mut self.humanoids, self.world_w, &self.fg_tiles);

        // Humanoid input pass.
        update_humanoid_inputs(&mut self.humanoids);

        // Humanoid physics pass.
        update_humanoid_physics(&mut self.humanoids, ft);

        // Humanoid tile collision pass.
        resolve_humanoid_tile_collisions(&mut self.humanoids, self.world_w, &self.fg_tiles);
    }

    pub fn poststep(&mut self, _ts: u64) {
        let humanoid_se = serialize(&[ServerNetMessage::HumanoidSync {
            humanoids: self.humanoids.clone(),
        }]);

        // Da big sink
        for (destination, connection) in self.connections.iter() {
            if connection.disconnect {
                //self.net_manager.send_uu()
                continue;
            }

            if !connection.joined {
                continue;
            }
            self.net_manager.send_uu(destination, humanoid_se.clone());
        }

        // Clean disconnects.
        self.connections.retain(|_, con| con.disconnect == false);

        // Poll for event sending.
        self.net_manager.poll();
    }

    fn handle_net_events(&mut self, _ts: u64) {
        for e in self.net_manager.recv() {
            let source = e.source;
            let bytes = match e.kind {
                NetEventKind::Data(bytes) => bytes,
                NetEventKind::Connect => {
                    self.connections.insert(
                        source,
                        Connection {
                            joined: false,
                            disconnect: false,
                            id: None,
                        },
                    );
                    continue;
                }
                NetEventKind::Disconnect => {
                    self.connections
                        .get_mut(&source)
                        .map(|con| con.disconnect = true);
                    continue;
                }
            };

            let msgs = deserialize(bytes).into_vec().into_iter();
            msgs.for_each(|msg| match msg {
                // Doesn't require a connection.
                ClientNetMessage::Connect { .. } => {
                    // Ignore.
                    if self.connections.contains_key(&source) {
                        log!("WARNING: {source:?} already connected: {msg:?}.");
                        return;
                    }

                    // Accept all connections.
                    self.net_manager
                        .send_ru(source, serialize(&[ServerNetMessage::ConnectAccept]));

                    log!("{source:?} has connected.");
                }

                // All further events require a connection.
                msg => {
                    let Some(connection) = self.connections.get_mut(&source) else {
                        log!("WARNING: {source:?} is not connected: {msg:?}");
                        return;
                    };

                    if connection.disconnect {
                        return;
                    }

                    match msg {
                        ClientNetMessage::RequestChunk { x, y, seq } => {
                            let cx = x as usize;
                            let cy = y as usize;
                            let cur_seq = &mut self.chunk_seqs[cx + cy * self.world_w / CHUNK_SIZE];

                            // If the seq is the same, skip.
                            assert!(seq <= *cur_seq);
                            if seq == *cur_seq {
                                return;
                            }

                            // Clone the chunk.
                            let mut fg_tiles = [Tile::None; CHUNK_AREA];
                            let mut bg_tiles = [Tile::None; CHUNK_AREA];
                            for y in 0..CHUNK_SIZE {
                                for x in 0..CHUNK_SIZE {
                                    let src_index =
                                        x + cx * CHUNK_SIZE + (y + cy * CHUNK_SIZE) * self.world_w;
                                    let dst_index = x + y * CHUNK_SIZE;
                                    fg_tiles[dst_index] = self.fg_tiles[src_index];
                                    bg_tiles[dst_index] = self.bg_tiles[src_index];
                                }
                            }

                            // Send.
                            self.net_manager.send_ru(
                                source,
                                serialize(&[ServerNetMessage::ChunkSync {
                                    x,
                                    y,
                                    seq: *cur_seq,
                                    fg_tiles,
                                    bg_tiles,
                                }]),
                            );
                        }

                        ClientNetMessage::Join => {
                            let mut msgs = Vec::new();

                            // Arbitrary spawn point.
                            let spawn_x = 100 * TILE_SIZE;
                            let spawn_y = 100 * TILE_SIZE;
                            let viewport_w = 1920;
                            let viewport_h = 1080;

                            let id = self.humanoid_id_counter.next();
                            connection.id = Some(id);

                            self.humanoids.insert(
                                id,
                                Humanoid {
                                    base: HumanoidBase {
                                        x: spawn_x as f32,
                                        y: spawn_y as f32,
                                        w: 32. - 8.,
                                        h: 48. - 8.,
                                        flags: HUMANOID_ON_GROUND_BIT,
                                    },
                                    ai: HumanoidAi::Player,
                                    input: HumanoidInput::default(),
                                    physics: HumanoidPhysics {
                                        max_dx: 120.,
                                        ..Default::default()
                                    },
                                },
                            );

                            msgs.push(ServerNetMessage::JoinAccept {
                                width: self.world_w as u16,
                                height: self.world_h as u16,
                                id,
                                spawn_x: 100,
                                spawn_y: 100,
                            });

                            // Calculate load area.
                            const TILE_CHUNK_SIZE: usize = TILE_SIZE * CHUNK_SIZE;
                            let x1 = (spawn_x - viewport_w / 2) / TILE_CHUNK_SIZE;
                            let x2 =
                                (spawn_x + viewport_w / 2 + TILE_CHUNK_SIZE - 1) / TILE_CHUNK_SIZE;
                            let y1 = (spawn_y - viewport_h / 2) / TILE_CHUNK_SIZE;
                            let y2 =
                                (spawn_y + viewport_h / 2 + TILE_CHUNK_SIZE - 1) / TILE_CHUNK_SIZE;

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
                            connection.joined = true;
                        }

                        ClientNetMessage::JoinComplete => {
                            self.net_manager
                                .send_ru(source, serialize(&[ServerNetMessage::Ping]));
                        }

                        ClientNetMessage::SyncPlayer { player } => {
                            let Some(id) = connection.id else {
                                log!("{source:?} does not exist in this world!");
                                return;
                            };

                            if let Some(humanoid) = self.humanoids.get_mut(&id) {
                                *humanoid = player;
                            }
                        }

                        ClientNetMessage::Ping => self
                            .net_manager
                            .send_ru(source, serialize(&[ServerNetMessage::Ping])),

                        _ => log!("Unhandled net event from {source:?}: {msg:?}"),
                    }
                }
            });
        }
    }
}
