use crate::net::{NetEventKind, ServerNetManager};
use crate::server::log;
use crate::shared::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

pub struct Connection {
    // Whether the client has joined yet.
    joined: bool,

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

        let mut humanoid_id_counter = HumanoidId::new();
        let mut humanoids = HashMap::new();
        humanoids.insert(humanoid_id_counter.next(), Humanoid {
            x: (100 * TILE_SIZE) as f32,
            y: (85 * TILE_SIZE) as f32,
            w: 32.,
            h: 32.,
            ..Default::default()
        });

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
        
        // Humanoid physics stuff.
        for humanoid in self.humanoids.values_mut() {
            humanoid.flags &= !(HumanoidFlags::OnGround as u8);
            
            // Gravity.
            humanoid.ddy += 500.;

            let last_y = humanoid.y;
            update_humanoid_physics_y(humanoid, ft);
            resolve_humanoid_tile_collision_y(humanoid, last_y, self.world_w, &self.fg_tiles);
            humanoid.ddy = 0.;

            let last_x = humanoid.x;
            update_humanoid_physics_x(humanoid, ft);
            resolve_humanoid_tile_collision_x(humanoid, last_x, self.world_w, &self.fg_tiles);
            humanoid.ddx = 0.;
        }

    }

    pub fn poststep(&mut self, _ts: u64) {
        let humanoid_se = serialize(&[ServerNetMessage::HumanoidSync {
            humanoids: self.humanoids.clone(),
        }]);

        // Da big sink
        for (destination, connection) in self.connections.iter() {
            if !connection.joined {
                continue;
            }
            self.net_manager.send_uu(destination, humanoid_se.clone());
        }

        // Poll for event sending.
        self.net_manager.poll();
    }

    fn handle_net_events(&mut self, _ts: u64) {
        for e in self.net_manager.recv() {
            let source = e.source;
            let bytes = match e.kind {
                NetEventKind::Data(bytes) => bytes,
                NetEventKind::Connect => continue,
                NetEventKind::Disconnect => {
                    self.connections.remove(&source);
                    continue;
                }
            };

            let msgs = deserialize(bytes).into_vec().into_iter();
            msgs.for_each(|msg| match msg {
                // Doesn't require a connection.
                ClientNetMessage::Connect { .. } => {
                    // Ignore
                    if self.connections.contains_key(&source) {
                        log!("WARNING: {source:?} already connected: {msg:?}.");
                        return;
                    }

                    // Accept all connections.
                    self.net_manager
                        .send_ru(source, serialize(&[ServerNetMessage::ConnectAccept]));

                    // Push connection.
                    self.connections.insert(
                        source,
                        Connection {
                            joined: false,
                            id: None,
                        },
                    );
                    log!("{source:?} has connected.");
                }

                // All further events require a connection.
                msg => {
                    let Some(connection) = self.connections.get_mut(&source) else {
                        log!("WARNING: {source:?} is not connected: {msg:?}");
                        return;
                    };

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
                                    x: spawn_x as f32,
                                    y: spawn_y as f32,
                                    w: 16.,
                                    h: 32.,
                                    dx: 0.,
                                    dy: 0.,
                                    ddx: 0.,
                                    ddy: 0.,
                                    flags: HumanoidFlags::OnGround as u8,
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
