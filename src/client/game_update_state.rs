use crate::client::{GameRenderDesc, SpriteRenderDesc, TileRenderDesc};
use crate::net::{ClientNetManager, NetEventKind};
use crate::shared::*;
use crate::shared::{Tile, TILE_LIGHT_PROPERTIES, TILE_SIZE};
use crate::window::InputEvent;
use std::collections::HashMap;
use std::path::Path;

pub struct GameUpdateState {
    // Net manager.
    net_manager: ClientNetManager,

    //Input.
    window_width: usize,
    window_height: usize,
    mouse_x: usize,
    mouse_y: usize,
    mouse_x_rel: usize,
    mouse_y_rel: usize,
    //
    left_queue: usize,
    right_queue: usize,
    jump_queue: usize,

    // Viewport.
    viewport_x: usize,
    viewport_y: usize,
    viewport_w: usize,
    viewport_h: usize,

    // World.
    time: f32, // 0..1

    // Collision.
    //detection_group: CollisionGroup<u8, ()>,
    //damage_group: CollisionGroup<u8, ()>,

    // Tiles.
    world_w: usize,
    world_h: usize,
    chunk_seqs: Box<[u32]>,
    fg_tiles: Box<[Tile]>,
    bg_tiles: Box<[Tile]>,

    // Humanoids.
    player_id: HumanoidId,
    humanoids: HashMap<HumanoidId, Humanoid>,
}

impl GameUpdateState {
    pub fn new(_root: &'static Path, mut net_manager: ClientNetManager) -> Self {
        let viewport_x = 32;
        let viewport_y = 32;
        let viewport_w = 1280;
        let viewport_h = 720;

        let mut spawn_x = 0;
        let mut spawn_y = 0;

        let mut world_w = 0;
        let mut world_h = 0;
        let mut chunk_seqs: Box<[u32]> = Box::new([]);
        let mut fg_tiles: Box<[Tile]> = Box::new([]);
        let mut bg_tiles: Box<[Tile]> = Box::new([]);

        let mut player_id = HumanoidId::new();
        let humanoids = HashMap::new();

        // Start join sequence.
        {
            // Initial join sequence.
            net_manager.send_ru(serialize(&[ClientNetMessage::Join]));
            net_manager.poll();

            'start: loop {
                // Get net messages.
                net_manager.poll();

                // Get all events.
                for net_event in net_manager.recv() {
                    match net_event.kind {
                        // Data net event.
                        NetEventKind::Data(bytes) => {
                            for msg in deserialize(bytes).to_vec() {
                                match msg {
                                    ServerNetMessage::JoinAccept {
                                        width,
                                        height,
                                        id,
                                        spawn_x: inner_spawn_x,
                                        spawn_y: inner_spawn_y,
                                    } => {
                                        // Player.
                                        player_id = id;

                                        // Init world.
                                        spawn_x = inner_spawn_x as usize;
                                        spawn_y = inner_spawn_y as usize;
                                        world_w = width as usize;
                                        world_h = height as usize;
                                        chunk_seqs = vec![0; world_w * world_h / CHUNK_AREA]
                                            .into_boxed_slice();
                                        fg_tiles =
                                            vec![Tile::None; world_w * world_h].into_boxed_slice();
                                        bg_tiles =
                                            vec![Tile::None; world_w * world_h].into_boxed_slice();
                                    }

                                    ServerNetMessage::ChunkSync {
                                        x,
                                        y,
                                        seq,
                                        fg_tiles: inner_fg_tiles,
                                        bg_tiles: inner_bg_tiles,
                                    } => {
                                        let cx = x as usize;
                                        let cy = y as usize;
                                        let cur_seq =
                                            &mut chunk_seqs[cx + cy * world_w / CHUNK_SIZE];

                                        // If the current sequence isn't less, skip this chunk.
                                        if *cur_seq >= seq {
                                            continue;
                                        }

                                        *cur_seq = seq;
                                        for y in 0..CHUNK_SIZE {
                                            for x in 0..CHUNK_SIZE {
                                                let src_index = x + y * CHUNK_SIZE;
                                                let dst_index = x
                                                    + cx * CHUNK_SIZE
                                                    + (y + cy * CHUNK_SIZE) * world_w;
                                                fg_tiles[dst_index] = inner_fg_tiles[src_index];
                                                bg_tiles[dst_index] = inner_bg_tiles[src_index];
                                            }
                                        }
                                    }

                                    ServerNetMessage::Start => {
                                        net_manager
                                            .send_ru(serialize(&[ClientNetMessage::JoinComplete]));
                                        break 'start;
                                    }

                                    _ => panic!("PANIC: {:?}", msg),
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
        }

        Self {
            // Net manager.
            net_manager,

            // Input.
            window_width: 0,
            window_height: 0,
            mouse_x: 0,
            mouse_y: 0,
            mouse_x_rel: 0,
            mouse_y_rel: 0,
            //
            left_queue: 0,
            right_queue: 0,
            jump_queue: 0,

            // Viewport.
            viewport_x,
            viewport_y,
            viewport_w,
            viewport_h,

            // World.
            time: 0.5,

            // Tiles.
            world_w,
            world_h,
            chunk_seqs,
            fg_tiles,
            bg_tiles,

            // Humanoids.
            player_id,
            humanoids,
        }
    }

    pub fn prestep(&mut self, ts: u64, input_events: impl Iterator<Item = InputEvent>) -> bool {
        // Shift all input queues.
        let shift = |queue: &mut _| *queue = *queue << 1 | *queue & 1;
        shift(&mut self.right_queue);
        shift(&mut self.left_queue);
        shift(&mut self.jump_queue);

        // Process input events.
        let exit = self.handle_input_events(ts, input_events);

        // Process net events.
        self.net_manager.poll();
        self.handle_net_events(ts);

        return exit;
    }

    pub fn step(&mut self, _ts: u64, ft: u64) {
        let ft = ft as f32 / 1e6;

        // Update time.
        let _day_cycle = 60.;
        //self.time += ft / day_cycle;
        if self.time > 1. {
            self.time = 0.;
        }

        // Player state stuff.
        if let Some(player) = self.humanoids.get_mut(&self.player_id) {
            player.input = HumanoidInput {
                jump_queue: self.jump_queue as u8,
                left_queue: self.left_queue as u8,
                right_queue: self.right_queue as u8,
            };
        }

        // Humanoid input pass.
        update_humanoid_inputs(&mut self.humanoids);

        // Humanoid physics pass.
        update_humanoid_physics(&mut self.humanoids, ft);

        // Humanoid tile collision pass.
        resolve_humanoid_tile_collisions(&mut self.humanoids, self.world_w, &self.fg_tiles);

        // Clamp position (TODO: right-bottom world clamp).
        if let Some(player) = self.humanoids.get(&mut self.player_id) {
            self.viewport_x =
                ((player.base.x + player.base.w / 2.) as usize).saturating_sub(self.viewport_w / 2);
            self.viewport_y =
                ((player.base.y + player.base.h / 2.) as usize).saturating_sub(self.viewport_h / 2);
        }
        self.viewport_x = std::cmp::max(2 * TILE_SIZE, self.viewport_x);
        self.viewport_y = std::cmp::max(2 * TILE_SIZE, self.viewport_y);
    }

    pub fn poststep(&mut self, _ts: u64) -> GameRenderDesc {
        // Send the server RequestChunk messages based on view.
        request_chunks_from_server(self);

        // Send the server the player's current state.
        if let Some(player) = self.humanoids.get(&self.player_id) {
            let bytes = serialize(&[ClientNetMessage::SyncPlayer {
                player: player.clone(),
            }]);
            self.net_manager.send_uu(bytes);
        }

        // Calculate light map.
        let (light_x, light_y, light_w, light_h, r, g, b) = calculate_light_map(self);

        // Clone the tiles in the visible range (plus 1).
        let (tiles_x, tiles_y, tiles_w, tiles_h, fg_tiles, bg_tiles) = clone_visible_tile_map(self);

        // Clone the sprites in the visible range..
        let sprites = clone_visible_sprites(self);

        // Poll the network to send all messages.
        self.net_manager.poll();

        // Pass the game render desc to the renderer.
        GameRenderDesc {
            viewport_x: self.viewport_x as f32,
            viewport_y: self.viewport_y as f32,
            viewport_w: self.viewport_w as f32,
            viewport_h: self.viewport_h as f32,

            sprites,

            light_x,
            light_y,
            light_w,
            light_h,
            r_channel: r,
            g_channel: g,
            b_channel: b,

            tiles_x,
            tiles_y,
            tiles_w,
            tiles_h,
            fg_tiles,
            bg_tiles,
        }
    }

    fn handle_net_events(&mut self, _ts: u64) {
        for e in self.net_manager.recv() {
            match e.kind {
                NetEventKind::Data(bytes) => {
                    for msg in deserialize(bytes).to_vec() {
                        match msg {
                            ServerNetMessage::ChunkSync {
                                x,
                                y,
                                seq,
                                fg_tiles,
                                bg_tiles,
                            } => {
                                let cx = x as usize;
                                let cy = y as usize;
                                let cur_seq =
                                    &mut self.chunk_seqs[cx + cy * self.world_w / CHUNK_SIZE];

                                if !(*cur_seq < seq) {
                                    continue;
                                }

                                *cur_seq = seq;
                                for y in 0..CHUNK_SIZE {
                                    for x in 0..CHUNK_SIZE {
                                        let src_index = x + y * CHUNK_SIZE;
                                        let dst_index = x
                                            + cx * CHUNK_SIZE
                                            + (y + cy * CHUNK_SIZE) * self.world_w;
                                        self.fg_tiles[dst_index] = fg_tiles[src_index];
                                        self.bg_tiles[dst_index] = bg_tiles[src_index];
                                    }
                                }
                            }

                            ServerNetMessage::HumanoidSync { humanoids } => {
                                // Clone player (if it can be found).
                                let player = self.humanoids.get(&self.player_id).cloned();

                                // Swap.
                                self.humanoids = humanoids;

                                // Put player back in.
                                if let Some(player) = player {
                                    self.humanoids.get_mut(&self.player_id).map(|p| *p = player);
                                }
                            }

                            ServerNetMessage::Ping => self
                                .net_manager
                                .send_ru(serialize(&[ClientNetMessage::Ping])),

                            _ => panic!("Uncaught event: {msg:?}."),
                        }
                    }
                }
                _ => panic!("Uncaught net event: {e:?}."),
            }
        }
    }

    fn handle_input_events(
        &mut self,
        _ts: u64,
        input_events: impl Iterator<Item = InputEvent>,
    ) -> bool {
        for e in input_events {
            use crate::window::*;
            match e {
                InputEvent::WindowClose => return true,
                InputEvent::WindowResize { width, height } => {
                    self.window_width = width as usize;
                    self.window_height = height as usize;
                }
                InputEvent::KeyboardInput {
                    keycode,
                    press_state,
                } => {
                    let bit = match press_state {
                        PressState::Up => 0,
                        PressState::Down => 1,
                        PressState::DownRepeat => 1,
                    };
                    match keycode {
                        'd' | 'D' => self.right_queue = self.right_queue & !1 | bit,
                        'a' | 'A' => self.left_queue = self.left_queue & !1 | bit,
                        ' ' => self.jump_queue = self.jump_queue & !1 | bit,
                        '1' if bit == 0 => {
                            let index = self.mouse_x / 16 + self.mouse_y / 16 * self.world_w;
                            self.fg_tiles[index] = Tile::RedTorch;
                        }
                        '2' if bit == 0 => {
                            let index = self.mouse_x / 16 + self.mouse_y / 16 * self.world_w;
                            self.fg_tiles[index] = Tile::GreenTorch;
                        }
                        '3' if bit == 0 => {
                            let index = self.mouse_x / 16 + self.mouse_y / 16 * self.world_w;
                            self.fg_tiles[index] = Tile::BlueTorch;
                        }
                        _ => {}
                    };
                }

                InputEvent::MouseMove { x, y } => {
                    let (x, y) = (x / self.window_width as f32, y / self.window_height as f32);
                    self.mouse_x_rel = (x * self.viewport_w as f32) as usize;
                    self.mouse_y_rel = (y * self.viewport_h as f32) as usize;
                    self.mouse_x = self.viewport_x + self.mouse_x_rel;
                    self.mouse_y = self.viewport_y + self.mouse_y_rel;
                }

                InputEvent::MouseClick {
                    mouse_button,
                    press_state,
                } => match (mouse_button, press_state) {
                    (MouseButton::Left | MouseButton::Right, PressState::Down) => {
                        let index = self.mouse_x / 16 + self.mouse_y / 16 * self.world_w;
                        match mouse_button {
                            MouseButton::Left => self.fg_tiles[index] = Tile::None,
                            MouseButton::Right => self.bg_tiles[index] = Tile::None,
                            _ => unreachable!(),
                        }
                    }
                    _ => {}
                },
            }
        }

        false
    }
}

fn request_chunks_from_server(game: &mut GameUpdateState) {
    const TILE_CHUNK_SIZE: usize = TILE_SIZE * CHUNK_SIZE;
    let cx = game.viewport_x + game.viewport_w / 2;
    let cy = game.viewport_y + game.viewport_h / 2;
    let x1 = (cx.saturating_sub(game.viewport_w / 2)) / TILE_CHUNK_SIZE;
    let x2 = (cx + game.viewport_w / 2 + TILE_CHUNK_SIZE - 1) / TILE_CHUNK_SIZE;
    let y1 = (cy.saturating_sub(game.viewport_h / 2)) / TILE_CHUNK_SIZE;
    let y2 = (cy + game.viewport_h / 2 + TILE_CHUNK_SIZE - 1) / TILE_CHUNK_SIZE;

    let mut msgs = vec![];
    for y in y1..y2 {
        for x in x1..x2 {
            msgs.push(ClientNetMessage::RequestChunk {
                x: x as u16,
                y: y as u16,
                seq: game.chunk_seqs[x + y * game.world_w / CHUNK_SIZE],
            });
        }
    }

    let _bytes = serialize(&msgs);
    //log!("{}", bytes.len());
    game.net_manager.send_uu(serialize(&msgs));
}

fn calculate_light_map(
    game: &mut GameUpdateState,
) -> (usize, usize, usize, usize, Box<[u8]>, Box<[u8]>, Box<[u8]>) {
    // Calculate sky light value.
    let (sky_r, sky_g, sky_b) = 'out: {
        // Morning.
        if game.time < 7. / 24. {
            break 'out (10, 10, 10);
        }

        // Day.
        if game.time < 18. / 24. {
            break 'out (40, 40, 40);
        }

        break 'out (10, 10, 10);
    };

    // Light lookup.
    let tile_light_property_map = &TILE_LIGHT_PROPERTIES;

    // Calculate visible region.
    let x1 = (game.viewport_x / 16).saturating_sub(LIGHT_MAX as usize);
    let y1 = (game.viewport_y / 16).saturating_sub(LIGHT_MAX as usize);
    let x2 = (game.viewport_x + game.viewport_w + 15) / 16 + LIGHT_MAX as usize;
    let y2 = (game.viewport_y + game.viewport_h + 15) / 16 + LIGHT_MAX as usize;
    let (w, h) = (x2 - x1, y2 - y1);

    let mut r_channel = create_light_map_base(w, h);
    let mut g_channel = create_light_map_base(w, h);
    let mut b_channel = create_light_map_base(w, h);
    let mut fade_map = create_fade_map_base(w, h);

    let mut r_probes = Vec::with_capacity(1024);
    let mut g_probes = Vec::with_capacity(1024);
    let mut b_probes = Vec::with_capacity(1024);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let world_index = (x + x1) + (y + y1) * game.world_w;
            let light_index = x + y * w;

            let fg_tile = game.fg_tiles[world_index];
            let bg_tile = game.bg_tiles[world_index];

            // Special case (None, None).
            if fg_tile == Tile::None && bg_tile == Tile::None {
                r_channel[light_index] = sky_r;
                g_channel[light_index] = sky_g;
                b_channel[light_index] = sky_b;
                r_probes.push(light_index as u16);
                g_probes.push(light_index as u16);
                b_probes.push(light_index as u16);
                continue;
            }

            // Special case (None, Some).
            if fg_tile == Tile::None && bg_tile != Tile::None {
                fade_map[light_index] = FADE_MIN;
                continue;
            }

            // Case (Some, _).
            if fg_tile != Tile::None {
                let fg_light_property = tile_light_property_map[fg_tile as usize];

                //
                fade_map[light_index] = fg_light_property.fade;

                //
                let (r, g, b) = fg_light_property.light;
                if r > 0 {
                    r_channel[light_index] = r;
                    r_probes.push(light_index as u16);
                }
                if g > 0 {
                    g_channel[light_index] = g;
                    g_probes.push(light_index as u16);
                }
                if b > 0 {
                    b_channel[light_index] = b;
                    b_probes.push(light_index as u16);
                }
                continue;
            }
        }
    }

    fill_light_map(w, &mut r_channel, &fade_map, r_probes);
    fill_light_map(w, &mut g_channel, &fade_map, g_probes);
    fill_light_map(w, &mut b_channel, &fade_map, b_probes);

    (x1, y1, w, h, r_channel, g_channel, b_channel)
}

fn clone_visible_tile_map(
    game: &mut GameUpdateState,
) -> (
    usize,
    usize,
    usize,
    usize,
    Box<[TileRenderDesc]>,
    Box<[TileRenderDesc]>,
) {
    let x1 = (game.viewport_x - 4) / 16 - 1;
    let y1 = (game.viewport_y - 4) / 16 - 1;
    let x2 = (game.viewport_x + game.viewport_w + 4 + 15) / 16 + 1;
    let y2 = (game.viewport_y + game.viewport_h + 4 + 15) / 16 + 1;
    let mut fg_tiles = vec![TileRenderDesc(Tile::None); (x2 - x1) * (y2 - y1)].into_boxed_slice();
    let mut bg_tiles = vec![TileRenderDesc(Tile::None); (x2 - x1) * (y2 - y1)].into_boxed_slice();
    let w = x2 - x1;
    let h = y2 - y1;
    for y in 0..h {
        for x in 0..w {
            let src_index = (x + x1) + (y + y1) * game.world_w;
            let dst_index = x + y * w;
            fg_tiles[dst_index] = TileRenderDesc(game.fg_tiles[src_index]);
            bg_tiles[dst_index] = TileRenderDesc(game.bg_tiles[src_index]);
        }
    }
    (x1, y1, x2 - x1, y2 - y1, fg_tiles, bg_tiles)
}

fn clone_visible_sprites(game: &mut GameUpdateState) -> Box<[SpriteRenderDesc]> {
    game.humanoids
        .values()
        .map(|humanoid| &humanoid.base)
        .map(|base| SpriteRenderDesc {
            x: base.x.floor(),
            y: base.y.floor(),
            w: base.w,
            h: base.h,
            u: 0.,
            v: 0.,
        })
        .collect()
}
