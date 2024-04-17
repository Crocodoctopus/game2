use crate::tile::*;
use bitcode::{Decode, Encode};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, Encode, Decode, Hash)]
pub struct HumanoidId(u32);

impl HumanoidId {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn next(&mut self) -> HumanoidId {
        self.0 += 1;
        return HumanoidId(self.0 - 1);
    }
}

pub type HumanoidFlags = u8;
pub const HUMANOID_ON_GROUND_BIT: u8 = 1 << 1;

#[derive(Clone, Debug, Encode, Decode)]
pub struct Humanoid {
    pub base: HumanoidBase,
    pub physics: HumanoidPhysics,
    pub ai: HumanoidAI,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct HumanoidBase {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub flags: u8,
}

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct HumanoidPhysics {
    pub last_x: f32,
    pub last_y: f32,
    pub dx: f32,
    pub dy: f32,
    pub ddx: f32,
    pub ddy: f32,
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum HumanoidAI {
    Player,
    Zombie,
}

pub struct HumanoidAnimation { }

pub fn update_humanoid_physics_x(base: &mut HumanoidBase, physics: &mut HumanoidPhysics, ft: f32) {
    physics.last_x = base.x;
    base.x += 0.5 * physics.ddx * ft * ft + physics.dx * ft;
    physics.dx += physics.ddx * ft;
}

pub fn resolve_humanoid_tile_collision_x(
    base: &mut HumanoidBase,
    physics: &mut HumanoidPhysics,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    // Calculate (x1..x2) based on distance moved.
    let (x1, x2) = if base.x > physics.last_x {
        let x1 = ((physics.last_x + base.w) / TILE_SIZE as f32).ceil() as usize;
        let x2 = ((base.x + base.w) / TILE_SIZE as f32).ceil() as usize;
        (x1, x2)
    } else {
        let x1 = base.x as usize / TILE_SIZE;
        let x2 = physics.last_x as usize / TILE_SIZE;
        (x1, x2)
    };

    // Calculate (y1..y2).
    let y1 = physics.last_y as usize / TILE_SIZE;
    let y2 = ((physics.last_y + base.h) / TILE_SIZE as f32).ceil() as usize;

    // Iterate all newly touched tiles.
    for y in y1..y2 {
        for x in x1..x2 {
            let src_index = x + y * stride;
            let tile = tiles[src_index];
            let property = TILE_PHYSICS_PROPERTIES[tile as usize]; // TODO pass this in?

            // Solid.
            if property.solid {
                if base.x > physics.last_x {
                    base.x = (x * TILE_SIZE) as f32 - base.w;
                }
                if base.x < physics.last_x {
                    base.x = ((x + 1) * TILE_SIZE) as f32;
                }
                physics.dx = 0.;
            }
        }
    }
}

pub fn update_humanoid_physics_y(base: &mut HumanoidBase, physics: &mut HumanoidPhysics, ft: f32) {
    physics.last_y = base.y;
    base.y += 0.5 * physics.ddy * ft * ft + physics.dy * ft;
    physics.dy += physics.ddy * ft;
}

pub fn resolve_humanoid_tile_collision_y(
    base: &mut HumanoidBase,
    physics: &mut HumanoidPhysics,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    // Calculate (x1..x2).
    let x1 = physics.last_x as usize / TILE_SIZE;
    let x2 = ((physics.last_x + base.w) / TILE_SIZE as f32).ceil() as usize;

    // Calculate (y1..y2) based on distance moved.
    let (y1, y2) = if base.y > physics.last_y {
        let y1 = ((physics.last_y + base.h) / TILE_SIZE as f32).ceil() as usize;
        let y2 = ((base.y + base.h) / TILE_SIZE as f32).ceil() as usize;
        (y1, y2)
    } else {
        let y1 = base.y as usize / TILE_SIZE;
        let y2 = physics.last_y as usize / TILE_SIZE;
        (y1, y2)
    };

    // Iterate all newly touched tiles.
    for y in y1..y2 {
        for x in x1..x2 {
            let src_index = x + y * stride;
            let tile = tiles[src_index];
            let property = TILE_PHYSICS_PROPERTIES[tile as usize]; // TODO pass this in?

            // Solid.
            if property.solid {
                if base.y > physics.last_y {
                    base.flags |= HUMANOID_ON_GROUND_BIT; 
                    base.y = (y * TILE_SIZE) as f32 - base.h;
                    physics.dy = 0.;
                }
                if base.y < physics.last_y {
                    base.y = ((y + 1) * TILE_SIZE) as f32;
                    physics.dy *= 0.50;
                }
            }
        }
    }
}
