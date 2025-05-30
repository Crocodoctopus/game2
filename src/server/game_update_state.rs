use crate::net::{NetEventKind, ServerNetManager};
use crate::server::log;
use crate::shared::GlobalId;
use crate::shared::humanoid::*;
use crate::shared::item::*;
use crate::shared::misc::Aabb;
use crate::shared::net::*;
use crate::shared::physics::*;
use crate::shared::tile::*;
use crate::shared::tile_collision::*;
use crate::shared::tile_damage::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

pub struct Connection {
    // Whether the client has joined yet.
    joined: bool,

    // Disconnect flag.
    disconnect: bool,

    // The ID this connection owns.
    id: Option<GlobalId>,
    // Chunk Reqs in flight
}

pub struct GameUpdateState {
    // State.
    id_counter: GlobalId,

    // Net manager.
    net_manager: ServerNetManager,
    connections: HashMap<SocketAddr, Connection>,

    // Tiles.
    fg_tiles: TileMap,
    bg_tiles: TileMap,
    tile_damages: HashMap<(u16, u16), TileDamage>,

    // Players.
    humanoids: HashMap<GlobalId, Humanoid>,

    // Floor items.
    items: HashMap<GlobalId, Item>,
}

impl GameUpdateState {
    pub fn new(_root: &'static Path, net_manager: ServerNetManager) -> Self {
        let world_w = 8400;
        let world_h = 2400;
        let mut fg_tiles = vec![TileKind::None; world_w * world_h].into_boxed_slice();
        let mut bg_tiles = vec![TileKind::None; world_w * world_h].into_boxed_slice();
        for y in 0..world_h {
            for x in 0..world_w {
                let index = x + y * world_w;

                if y == 0 || y == world_h - 1 || x == 0 || x == world_w - 1 {
                    fg_tiles[index] = TileKind::Dirt;
                    bg_tiles[index] = TileKind::Dirt;
                    continue;
                }

                if y < 102 {
                    fg_tiles[index] = TileKind::None;
                    bg_tiles[index] = TileKind::None;
                    continue;
                }

                if y < 102 + 5 {
                    fg_tiles[index] = TileKind::Dirt;
                    bg_tiles[index] = TileKind::Dirt;
                    continue;
                }

                if y < 102 + 15 {
                    fg_tiles[index] = TileKind::Stone;
                    bg_tiles[index] = TileKind::Stone;
                    continue;
                }

                fg_tiles[index] = TileKind::DenseStone;
                bg_tiles[index] = TileKind::DenseStone;
            }
        }
        fg_tiles[108 + 102 * world_w] = TileKind::None;
        fg_tiles[109 + 102 * world_w] = TileKind::None;
        fg_tiles[107 + 102 * world_w] = TileKind::None;
        fg_tiles[106 + 102 * world_w] = TileKind::None;
        fg_tiles[107 + 103 * world_w] = TileKind::None;
        fg_tiles[106 + 103 * world_w] = TileKind::None;

        let mut id_counter = GlobalId::new();

        let mut humanoids = HashMap::new();
        let spawn_x = 100 * TILE_SIZE;
        let spawn_y = 100 * TILE_SIZE - 32;

        // Zomble
        humanoids.insert(
            id_counter.next(),
            Humanoid {
                bounds: Aabb {
                    x: spawn_x as f32 + 256.,
                    y: spawn_y as f32,
                    width: 32. - 8.,
                    height: 48. - 8.,
                },
                last_x: spawn_x as f32 + 256.,
                last_y: spawn_y as f32,
                flags: tile_collision_flags::NONE,
                ai: HumanoidAi::Zombie,
                input: HumanoidInput {
                    ..Default::default()
                },
                physics: GenericPhysics {
                    ..Default::default()
                },
            },
        );

        Self {
            id_counter,

            net_manager,
            connections: HashMap::new(),

            fg_tiles: TileMap::from_data(world_w, world_h, fg_tiles),
            bg_tiles: TileMap::from_data(world_w, world_h, bg_tiles),
            tile_damages: <_>::default(),

            humanoids,

            items: HashMap::new(),
        }
    }

    pub fn prestep(&mut self, timestamp: u64) {
        // Poll for event receiving.
        self.net_manager.poll();
        self.handle_net_events(timestamp);
    }

    pub fn step(&mut self, timestamp: u64, frametime: u64) {
        let frametime = frametime as f32 / 1e6;

        // Humanoid AI pass.
        update_humanoid_ais(&mut self.humanoids, &self.fg_tiles);

        // Humanoid input pass.
        update_humanoid_inputs(&mut self.humanoids);

        // Humanoid physics and collision pass.
        update_humanoid_physics(&mut self.humanoids, frametime, &self.fg_tiles);

        // Process tile damages.
        let destroyed_tiles = update_tile_damages(&mut self.tile_damages, timestamp);

        // Spawn items from destroyed tiles.
        for (x, y) in &destroyed_tiles {
            let (x, y) = (*x as usize, *y as usize);
            let stride = self.fg_tiles.width();
            self.items.insert(
                self.id_counter.next(),
                Item {
                    bounds: Aabb {
                        x: (x * TILE_SIZE) as f32,
                        y: (y * TILE_SIZE) as f32,
                        width: 16.,
                        height: 16.,
                    },
                    last_x: (x * TILE_SIZE) as f32,
                    last_y: (y * TILE_SIZE) as f32,
                    physics: GenericPhysics {
                        dx: 0.,
                        dy: 0.,
                        ddx: 0.,
                        ddy: 0.,
                    },
                    flags: 0,
                    kind: ItemKind::Tile(self.fg_tiles[x + y * stride]),
                    count: 1,
                },
            );
        }

        //
        update_item_physics(&mut self.items, frametime, &self.fg_tiles);

        // Temp tile sync stuff
        {
            // Temp
            let tiles_se = serialize(
                &destroyed_tiles
                    .into_iter()
                    .map(|(x, y)| {
                        let index = x as usize + y as usize * self.fg_tiles.width();
                        self.fg_tiles[index] = TileKind::None;
                        ServerNetMessage::TileSync {
                            x,
                            y,
                            tile: TileKind::None,
                        }
                    })
                    .collect::<Box<[_]>>(),
            );

            // Deff temp
            for (destination, _) in self.connections.iter() {
                self.net_manager.send_ru(destination, tiles_se.clone())
            }
        }
    }

    pub fn poststep(&mut self, _timestamp: u64) {
        let humanoid_se = serialize(&[ServerNetMessage::HumanoidSync {
            humanoids: self
                .humanoids
                .iter()
                .map(|(k, v)| (*k, NetHumanoid(v.clone())))
                .collect(),
        }]);

        let items_se = serialize(&[ServerNetMessage::ItemSync {
            items: self
                .items
                .iter()
                .map(|(id, item)| {
                    (
                        *id,
                        NetItem {
                            kind: item.kind,
                            x: item.bounds.x,
                            y: item.bounds.y,
                        },
                    )
                })
                .collect(),
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
            self.net_manager.send_uu(destination, items_se.clone());
        }

        // Clean disconnects.
        self.connections.retain(|_, con| !con.disconnect);

        // Poll for event sending.
        self.net_manager.poll();
    }

    fn handle_net_events(&mut self, timestamp: u64) {
        for e in self.net_manager.recv() {
            let source = e.source;
            let bytes = match e.kind {
                NetEventKind::Data(bytes) => bytes,
                NetEventKind::Connect => {
                    log!("User: {} has connected.", source);
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
                    log!("User: {} has disconnected.", source);
                    if let Some(con) = self.connections.get_mut(&source) {
                        con.disconnect = true;
                    }
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
                        ClientNetMessage::RequestChunk { x, y } => {
                            let cx = x as usize;
                            let cy = y as usize;

                            // Clone the chunk.
                            let mut fg_tiles = [TileKind::None; CHUNK_AREA];
                            let mut bg_tiles = [TileKind::None; CHUNK_AREA];
                            for y in 0..CHUNK_SIZE {
                                for x in 0..CHUNK_SIZE {
                                    let src_index = x
                                        + cx * CHUNK_SIZE
                                        + (y + cy * CHUNK_SIZE) * self.fg_tiles.width();
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

                            let id = self.id_counter.next();
                            connection.id = Some(id);

                            self.humanoids.insert(
                                id,
                                Humanoid {
                                    bounds: Aabb {
                                        x: spawn_x as f32,
                                        y: spawn_y as f32,
                                        width: 32. - 8.,
                                        height: 48. - 8.,
                                    },
                                    last_x: spawn_x as f32,
                                    last_y: spawn_y as f32,
                                    flags: tile_collision_flags::NONE,
                                    ai: HumanoidAi::Player,
                                    input: HumanoidInput {
                                        ..Default::default()
                                    },
                                    physics: GenericPhysics {
                                        ..Default::default()
                                    },
                                },
                            );

                            msgs.push(ServerNetMessage::JoinAccept {
                                width: self.fg_tiles.width() as u16,
                                height: self.fg_tiles.height() as u16,
                                id,
                                spawn_x: 100,
                                spawn_y: 100,
                            });

                            // Calculate load area.
                            const TILE_CHUNK_SIZE: usize = TILE_SIZE * CHUNK_SIZE;
                            let x1 = (spawn_x - viewport_w / 2) / TILE_CHUNK_SIZE;
                            let x2 = (spawn_x + viewport_w / 2).div_ceil(TILE_CHUNK_SIZE);
                            let y1 = (spawn_y - viewport_h / 2) / TILE_CHUNK_SIZE;
                            let y2 = (spawn_y + viewport_h / 2).div_ceil(TILE_CHUNK_SIZE);

                            // Send chunk data.
                            for cy in y1..y2 {
                                for cx in x1..x2 {
                                    let offset = (cx + cy * self.fg_tiles.width()) * CHUNK_SIZE;
                                    let mut fg_tiles = [TileKind::None; CHUNK_AREA];
                                    let mut bg_tiles = [TileKind::None; CHUNK_AREA];
                                    for y in 0..CHUNK_SIZE {
                                        for x in 0..CHUNK_SIZE {
                                            let src_index = x + y * self.fg_tiles.width();
                                            let dst_index = x + y * CHUNK_SIZE;
                                            fg_tiles[dst_index] = self.fg_tiles[src_index + offset];
                                            bg_tiles[dst_index] = self.bg_tiles[src_index + offset];
                                        }
                                    }

                                    msgs.push(ServerNetMessage::ChunkSync {
                                        x: cx as u16,
                                        y: cy as u16,
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

                        ClientNetMessage::HitTile { x, y } => {
                            let tile = self.fg_tiles[(x as usize, y as usize)];
                            register_tile_hit(&mut self.tile_damages, x, y, tile, timestamp);
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
