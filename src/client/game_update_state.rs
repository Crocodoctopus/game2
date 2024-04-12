use crate::client::{GameRenderDesc, SpriteRenderDesc, TileRenderDesc};
use crate::shared::*;
use crate::shared::{Tile, TILE_LIGHT_PROPERTIES, TILE_SIZE};
use crate::InputEvent;
use std::path::Path;

pub struct GameUpdateState {
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

    // Tiles.
    world_w: usize,
    world_h: usize,
    fg_tiles: Box<[Tile]>,
    bg_tiles: Box<[Tile]>,

    // Humanoids.
    humanoids: Vec<Humanoid>,
}

impl GameUpdateState {
    pub fn new(root: &'static Path) -> Self {
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

        // Temp.
        let humanoids = vec![Humanoid {
            x: 64.,
            y: 64.,
            w: 16.,
            h: 24.,
            ..Default::default()
        }];

        Self {
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
            viewport_x: 32,
            viewport_y: 32,
            viewport_w: 1280,
            viewport_h: 720,

            // World.
            time: 0.,

            // Tiles.
            world_w,
            world_h,
            fg_tiles,
            bg_tiles,

            // Humanoids.
            humanoids,
        }
    }

    pub fn prestep<'a>(
        &mut self,
        ts: u64,
        input_events: impl Iterator<Item = &'a InputEvent>,
    ) -> bool {
        let shift = |queue: &mut _| *queue = *queue << 1 | *queue & 1;
        shift(&mut self.right_queue);
        shift(&mut self.left_queue);
        shift(&mut self.jump_queue);

        for &e in input_events {
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
                } => {
                    println!("{mouse_button:?}, {press_state:?}");
                    match (mouse_button, press_state) {
                        (MouseButton::Left | MouseButton::Right, PressState::Down) => {
                            let index = self.mouse_x / 16 + self.mouse_y / 16 * self.world_w;
                            match mouse_button {
                                MouseButton::Left => self.fg_tiles[index] = Tile::None,
                                MouseButton::Right => self.bg_tiles[index] = Tile::None,
                                _ => unreachable!(),
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        false
    }

    pub fn step(&mut self, ts: u64, ft: u64) {
        let ft = ft as f32 / 1e6;

        // Update time.
        let day_cycle = 60.;
        self.time += ft / day_cycle;
        if self.time > 1. {
            self.time = 0.;
        }

        // Player state stuff.
        if let Some(player) = self.humanoids.get_mut(0) {
            player.ddy += 500.;
            if self.right_queue & 1 != 0 {
                player.ddx += 1500.;
            }
            if self.left_queue & 1 != 0 {
                player.ddx -= 1500.;
            }

            // Check if jump was pressed at all during the last 3 frames.
            let jump_buffer = (0..3)
                .into_iter()
                .map(|i| self.jump_queue >> i & 0b11 == 0b01)
                .reduce(|b, acc| acc | b)
                .unwrap();

            if jump_buffer && player.flags & (HumanoidFlags::OnGround as u8) != 0 {
                player.dy -= 300.;
            }

            // World's best friction.
            player.dx *= 0.80;
            if player.dx.abs() < 0.5 {
                player.dx = 0.;
            }
        }

        // Humanoid physics stuff.
        for humanoid in &mut self.humanoids {
            humanoid.flags &= !(HumanoidFlags::OnGround as u8);

            let last_y = humanoid.y;
            update_humanoid_physics_y(humanoid, ft);
            resolve_humanoid_tile_collision_y(humanoid, last_y, self.world_w, &self.fg_tiles);
            humanoid.ddy = 0.;

            let last_x = humanoid.x;
            update_humanoid_physics_x(humanoid, ft);
            resolve_humanoid_tile_collision_x(humanoid, last_x, self.world_w, &self.fg_tiles);
            humanoid.ddx = 0.;
        }

        // Clamp position (TODO: right-bottom world clamp).
        if let Some(player) = self.humanoids.get(0) {
            self.viewport_x =
                ((player.x + player.w / 2.) as usize).saturating_sub(self.viewport_w / 2);
            self.viewport_y =
                ((player.y + player.h / 2.) as usize).saturating_sub(self.viewport_h / 2);
        }
        self.viewport_x = std::cmp::max(2 * TILE_SIZE, self.viewport_x);
        self.viewport_y = std::cmp::max(2 * TILE_SIZE, self.viewport_y);
    }

    pub fn poststep(&mut self, ts: u64) -> GameRenderDesc {
        // Lighting.
        let (light_x, light_y, light_w, light_h, r_channel, g_channel, b_channel) = {
            // Calculate sky light value.
            let (sky_r, sky_g, sky_b) = 'out: {
                // Morning.
                if self.time < 7. / 24. {
                    break 'out (10, 10, 10);
                }

                // Day.
                if self.time < 18. / 24. {
                    break 'out (40, 40, 40);
                }

                break 'out (10, 10, 10);
            };

            // Light lookup.
            let tile_light_property_map = &TILE_LIGHT_PROPERTIES;

            // Calculate visible region.
            let x1 = (self.viewport_x / 16).saturating_sub(LIGHT_MAX as usize);
            let y1 = (self.viewport_y / 16).saturating_sub(LIGHT_MAX as usize);
            let x2 = (self.viewport_x + self.viewport_w + 15) / 16 + LIGHT_MAX as usize;
            let y2 = (self.viewport_y + self.viewport_h + 15) / 16 + LIGHT_MAX as usize;
            let (w, h) = (x2 - x1, y2 - y1);

            use crate::light::*;
            let mut r_channel = create_light_map_base(w, h);
            let mut g_channel = create_light_map_base(w, h);
            let mut b_channel = create_light_map_base(w, h);
            let mut fade_map = create_fade_map_base(w, h);

            let mut r_probes = Vec::with_capacity(1024);
            let mut g_probes = Vec::with_capacity(1024);
            let mut b_probes = Vec::with_capacity(1024);
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let world_index = (x + x1) + (y + y1) * self.world_w;
                    let light_index = x + y * w;

                    let fg_tile = self.fg_tiles[world_index];
                    let bg_tile = self.bg_tiles[world_index];

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
        };

        // Clone the tiles in the visible range (plus 1).
        let (tiles_x, tiles_y, tiles_w, tiles_h, fg_tiles, bg_tiles) = {
            let x1 = (self.viewport_x - 4) / 16 - 1;
            let y1 = (self.viewport_y - 4) / 16 - 1;
            let x2 = (self.viewport_x + self.viewport_w + 4 + 15) / 16 + 1;
            let y2 = (self.viewport_y + self.viewport_h + 4 + 15) / 16 + 1;
            let mut fg_tiles =
                vec![TileRenderDesc(Tile::None); (x2 - x1) * (y2 - y1)].into_boxed_slice();
            let mut bg_tiles =
                vec![TileRenderDesc(Tile::None); (x2 - x1) * (y2 - y1)].into_boxed_slice();
            let w = x2 - x1;
            let h = y2 - y1;
            for y in 0..h {
                for x in 0..w {
                    let src_index = (x + x1) + (y + y1) * self.world_w;
                    let dst_index = x + y * w;
                    fg_tiles[dst_index] = TileRenderDesc(self.fg_tiles[src_index]);
                    bg_tiles[dst_index] = TileRenderDesc(self.bg_tiles[src_index]);
                }
            }
            (x1, y1, x2 - x1, y2 - y1, fg_tiles, bg_tiles)
        };

        // Clone sprites.
        let sprites = self
            .humanoids
            .iter()
            .map(|humanoid| SpriteRenderDesc {
                x: humanoid.x.floor(),
                y: humanoid.y.floor(),
                w: humanoid.w,
                h: humanoid.h,
                u: 0.,
                v: 0.,
            })
            .collect();

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
            r_channel,
            g_channel,
            b_channel,

            tiles_x,
            tiles_y,
            tiles_w,
            tiles_h,
            fg_tiles,
            bg_tiles,
        }
    }
}
