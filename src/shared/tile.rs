use crate::shared::light::*;
use bitcode::{Decode, Encode};
use lazy_static::lazy_static;

pub const TILE_SIZE: usize = 16;
pub const TILE_BORDER_SIZE: usize = 4;

// Chunk.
pub const CHUNK_SIZE: usize = 8;
pub const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;

// View.
pub const CHUNK_LOAD_WIDTH: usize = 10;
pub const CHUNK_LOAD_HEIGHT: usize = 6;

pub struct TileMap {
    width: usize,
    height: usize,
    pub data: Box<[Tile]>,
}

impl std::ops::Index<usize> for TileMap {
    type Output = Tile;
    fn index(&self, i: usize) -> &Self::Output {
        &self.data[i]
    }
}

impl std::ops::IndexMut<usize> for TileMap {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.data[i]
    }
}

impl std::ops::Index<(usize, usize)> for TileMap {
    type Output = Tile;
    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.data[x + y * self.width]
    }
}

impl std::ops::IndexMut<(usize, usize)> for TileMap {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        &mut self.data[x + y * self.width]
    }
}

impl TileMap {
    pub fn from_data(width: usize, height: usize, data: Box<[Tile]>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum Tile {
    None = 0,
    Dirt,
    Stone,
    DenseStone,
    //
    RedTorch,
    GreenTorch,
    BlueTorch,
    WhiteTorch,

    // Last element
    SIZE,
}

const TILE_COUNT: usize = Tile::SIZE as usize;

lazy_static! {
    pub static ref TILE_LIGHT_PROPERTIES: [TileLightProperty; TILE_COUNT] =
        TileLightProperty::gen();
    pub static ref TILE_TEXTURE_PROPERTIES: [TileTextureProperty; TILE_COUNT] =
        TileTextureProperty::gen();
    pub static ref TILE_PHYSICS_PROPERTIES: [TilePhysicsProperty; TILE_COUNT] =
        TilePhysicsProperty::gen();
}

#[derive(Copy, Clone, Debug)]
pub struct TileLightProperty {
    pub fade: Opacity,
    pub light: (Brightness, Brightness, Brightness),
}

impl TileLightProperty {
    fn gen() -> [Self; TILE_COUNT] {
        // Generate default map.
        let mut map = [Self {
            fade: FADE_MIN,
            light: Brightness::clamp3(0, 0, 0),
        }; TILE_COUNT];

        // Fill.
        map[Tile::Dirt as usize] = Self {
            fade: FADE_SOLID,
            light: Brightness::clamp3(0, 0, 0),
        };
        map[Tile::Stone as usize] = Self {
            fade: FADE_SOLID,
            light: Brightness::clamp3(0, 0, 0),
        };
        map[Tile::DenseStone as usize] = Self {
            fade: FADE_DENSE,
            light: Brightness::clamp3(0, 0, 0),
        };
        map[Tile::RedTorch as usize] = Self {
            fade: FADE_MIN,
            light: Brightness::clamp3(255, 0, 0),
        };
        map[Tile::GreenTorch as usize] = Self {
            fade: FADE_MIN,
            light: Brightness::clamp3(0, 255, 0),
        };
        map[Tile::BlueTorch as usize] = Self {
            fade: FADE_MIN,
            light: Brightness::clamp3(0, 0, 255),
        };
        map[Tile::WhiteTorch as usize] = Self {
            fade: FADE_MIN,
            light: Brightness::clamp3(6, 38, 255),
        };

        map
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TileTextureProperty {
    pub u: f32,
    pub v: f32,
    pub depth: u8,
}

impl TileTextureProperty {
    fn gen() -> [Self; TILE_COUNT] {
        // Generate default map.
        let mut map = [Self {
            u: 0.,
            v: 0.,
            depth: 0,
        }; TILE_COUNT];

        // Fill.
        map[Tile::Dirt as usize] = Self {
            u: 16.,
            v: 0.,
            depth: 1,
        };
        map[Tile::Stone as usize] = Self {
            u: 32.,
            v: 0.,
            depth: 2,
        };
        map[Tile::DenseStone as usize] = Self {
            u: 48.,
            v: 0.,
            depth: 3,
        };

        return map;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TilePhysicsProperty {
    pub solid: bool,
}

impl TilePhysicsProperty {
    fn gen() -> [Self; TILE_COUNT] {
        // Generate default map.
        let mut map = [Self { solid: true }; TILE_COUNT];

        // Fill.
        map[Tile::None as usize] = Self { solid: false };

        return map;
    }
}
