use crate::shared::*;
use lazy_static::lazy_static;

pub const TILE_SIZE: usize = 16;
pub const TILE_BORDER_SIZE: usize = 4;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Tile {
    None = 0,
    Dirt,
    Stone,
    DenseStone,
    //
    RedTorch,
    GreenTorch,
    BlueTorch,

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
    pub fade: u8,
    pub light: (u8, u8, u8),
}

impl TileLightProperty {
    fn gen() -> [Self; TILE_COUNT] {
        // Generate default map.
        let mut map = [Self {
            fade: FADE_MIN,
            light: (0, 0, 0),
        }; TILE_COUNT];

        // Fill.
        map[Tile::Dirt as usize] = Self {
            fade: FADE_SOLID,
            light: (0, 0, 0),
        };
        map[Tile::Stone as usize] = Self {
            fade: FADE_SOLID,
            light: (0, 0, 0),
        };
        map[Tile::DenseStone as usize] = Self {
            fade: FADE_DENSE,
            light: (0, 0, 0),
        };
        map[Tile::RedTorch as usize] = Self {
            fade: FADE_MIN,
            light: (LIGHT_MAX - 10, 0, 0),
        };
        map[Tile::GreenTorch as usize] = Self {
            fade: FADE_MIN,
            light: (0, LIGHT_MAX - 10, 0),
        };
        map[Tile::BlueTorch as usize] = Self {
            fade: FADE_MIN,
            light: (0, 0, LIGHT_MAX - 10),
        };

        return map;
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
