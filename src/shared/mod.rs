pub mod entity_collision;
pub mod humanoid;
pub mod input;
pub mod item;
pub mod light;
pub mod misc;
pub mod net;
pub mod physics;
pub mod tile;
pub mod tile_collision;
pub mod tile_damage;

use bitcode::{Decode, Encode};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Encode, Decode, Hash)]
pub struct GlobalId(u32);

impl GlobalId {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn next(&mut self) -> Self {
        self.0 += 1;
        return Self(self.0 - 1);
    }
}
