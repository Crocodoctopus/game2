use crate::client::GameFrame;
use crate::InputEvent;
use std::path::Path;
use std::collections::HashSet;
use rand::Rng;

pub struct GameUpdateState {
    world_w: usize,
    world_h: usize,
    fg_tiles: Vec<u8>,
    bg_tiles: Vec<u8>,
    // TODO: abstract key presses into game events before this point
    // this is debug code
    just_pressed: HashSet<char>,
    just_released: HashSet<char>,

    trauma_percent: f32,
    trauma_last_updated_ts: u64,
}

impl GameUpdateState {
    pub fn new(root: &'static Path) -> Self {
        let world_w = 8400;
        let world_h = 2400;
        let mut fg_tiles = vec![0u8; world_w * world_h];
        let mut bg_tiles = vec![0u8; world_w * world_h];
        for y in 0..world_h {
            for x in 0..world_w {
                let index = x + y * world_w;

                if (x % 4 == 0 && y % 4 == 0) {
                    continue;
                }

                fg_tiles[index] = 1;
                bg_tiles[index] = 1;
                if y == 0 || y == world_h - 1 || x == 0 || x == world_w - 1 {
                    continue;
                }

                fg_tiles[index] = 3;
                bg_tiles[index] = 3;

                if y > 30 {
                    continue;
                }
                fg_tiles[index] = 2;
                bg_tiles[index] = 2;

                if y > 20 {
                    continue;
                }
                fg_tiles[index] = 1;
                bg_tiles[index] = 1;

                if y > 16 {
                    continue;
                }
                fg_tiles[index] = 0;
                bg_tiles[index] = 0;
            }
        }

        Self {
            world_w,
            world_h,
            fg_tiles,
            bg_tiles,
            just_pressed: Default::default(),
            just_released: Default::default(),
            trauma_percent: 0.,
            trauma_last_updated_ts: 0,
        }
    }

    pub fn prestep(&mut self, ts: u64, input_events: impl Iterator<Item = InputEvent>) -> bool {
        self.just_pressed.clear();
        self.just_released.clear();
        for e in input_events {
            match e {
                InputEvent::KeyboardInput {
                    keycode,
                    press_state,
                } => {
                    match press_state {
                        crate::PressState::Down => {
                            self.just_pressed.insert(keycode);
                        }
                        crate::PressState::Up => {
                            self.just_released.insert(keycode);
                        }
                        _ => ()
                    }
                    println!("{keycode:?} {press_state:?}")
                }
                _ => {}
            }
        }

        false
    }

    pub fn step(&mut self, ts: u64, ft: u64) {
        if self.just_pressed.contains(&'H') {
            self.trauma_percent += 10.;
            if self.trauma_percent > 100. {
                self.trauma_percent = 100.;
            }
            self.trauma_last_updated_ts = ts;
        }
        if self.trauma_percent > 0. {
            // TODO: adjust based on delta time later
            self.trauma_percent -= 0.3;
        }
    }

    pub fn poststep(&mut self, ts: u64) -> GameFrame {
        let viewport_x = 32_usize;
        let viewport_y = 32_usize;
        let viewport_w = 1920_usize;
        let viewport_h = 1080_usize;

        // Lighting
        let (light_x, light_y, light_w, light_h, r_channel, g_channel, b_channel) = {
            // Calculate visible region.
            let x1 = (viewport_x / 16).saturating_sub(LIGHT_MAX as usize);
            let y1 = (viewport_y / 16).saturating_sub(LIGHT_MAX as usize);
            let x2 = (viewport_x + viewport_w + 15) / 16 + LIGHT_MAX as usize;
            let y2 = (viewport_y + viewport_h + 15) / 16 + LIGHT_MAX as usize;
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

                    match (fg_tile, bg_tile) {
                        // Daylight
                        (0, 0) => {
                            r_channel[light_index] = LIGHT_MAX;
                            g_channel[light_index] = LIGHT_MAX;
                            b_channel[light_index] = LIGHT_MAX;
                            r_probes.push(light_index as u16);
                            g_probes.push(light_index as u16);
                            b_probes.push(light_index as u16);
                        }
                        // Dense FG
                        (3, _) => fade_map[light_index] = FADE_DENSE,
                        // Solid FG
                        (_, _) => fade_map[light_index] = FADE_SOLID,
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
            let x1 = (viewport_x - 4) / 16 - 1;
            let y1 = (viewport_y - 4) / 16 - 1;
            let x2 = (viewport_x + viewport_w + 4 + 15) / 16 + 1;
            let y2 = (viewport_y + viewport_h + 4 + 15) / 16 + 1;
            let mut fg_tiles = vec![0u8; (x2 - x1) * (y2 - y1)].into_boxed_slice();
            let mut bg_tiles = vec![0u8; (x2 - x1) * (y2 - y1)].into_boxed_slice();
            let w = x2 - x1;
            let h = y2 - y1;
            for y in 0..h {
                for x in 0..w {
                    let src_index = (x + x1) + (y + y1) * self.world_w;
                    let dst_index = x + y * w;
                    fg_tiles[dst_index] = self.fg_tiles[src_index];
                    bg_tiles[dst_index] = self.bg_tiles[src_index];
                }
            }
            (x1, y1, x2 - x1, y2 - y1, fg_tiles, bg_tiles)
        };

        let ca_offsets = {
            let mut ca_offsets = [[0., 0.], [0., 0.], [0., 0.]];
            if self.trauma_percent > 0. {
                let t = (ts - self.trauma_last_updated_ts) as f32 / 1_000_000.0;
                let mut rng = rand::thread_rng();
                let amt = self.trauma_percent as f32 / 100.0;
                for i in [0, 2] {
                    ca_offsets[i][0] = (rng.gen_range(-0.002..0.002) * amt) / t;
                    ca_offsets[i][1] = (rng.gen_range(-0.002..0.002) * amt) / t;
                }
            }
            ca_offsets
        };

        GameFrame {
            viewport_x: 32.,
            viewport_y: 32.,
            viewport_w: 1920.,
            viewport_h: 1080.,

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

            ca_offsets,
        }
    }
}
