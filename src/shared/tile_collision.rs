use crate::shared::misc::Aabb;
use crate::shared::tile::*;

pub type TileCollisionFlags = u8;
pub mod tile_collision_flags {
    type Flag = super::TileCollisionFlags;
    pub const NONE: Flag = 1 << 1;
    pub const HIT_CEILING: Flag = 1 << 2;
    pub const HIT_FLOOR: Flag = 1 << 3;
    pub const HIT_WALL: Flag = 1 << 4;
}

/*
pub enum GenericTileCollisionEvent {
    OnGround(f32, f32),
    WallHitLeft(f32, f32),
    WallHitRight(f32, f32),
}
*/

pub enum TileCollisionEvent {
    LeftWall,
    RightWall,
    Ceiling,
    Floor,
}

pub fn resolve_generic_tile_collision_x(
    bounds: Aabb,
    last_x: f32,
    last_y: f32,
    tiles: &TileMap,
    mut f: impl FnMut(TileCollisionEvent, usize, usize, &TileKind),
) {
    // Calculate (x1..x2) based on distance moved.
    let (x1, x2) = if bounds.x > last_x {
        let x1 = ((last_x + bounds.width) / TILE_SIZE as f32).ceil() as usize;
        let x2 = ((bounds.x + bounds.width) / TILE_SIZE as f32).ceil() as usize;
        (x1, x2)
    } else {
        let x1 = bounds.x as usize / TILE_SIZE;
        let x2 = last_x as usize / TILE_SIZE;
        (x1, x2)
    };

    // Calculate (y1..y2).
    let y1 = last_y as usize / TILE_SIZE;
    let y2 = ((last_y + bounds.height) / TILE_SIZE as f32).ceil() as usize;

    // Iterate all newly touched tiles.
    for y in y1..y2 {
        for x in x1..x2 {
            let tile = tiles[(x, y)];
            let property = TILE_PHYSICS_PROPERTIES[tile as usize]; // TODO pass this in?

            // Solid.
            if property.solid {
                // Right wall hit.
                if bounds.x > last_x {
                    f(TileCollisionEvent::RightWall, x, y, &tile);
                }
                // Left wall hit.
                if bounds.x < last_x {
                    f(TileCollisionEvent::LeftWall, x, y, &tile);
                }
            }
        }
    }
}

pub fn resolve_generic_tile_collision_y(
    bounds: Aabb,
    last_x: f32,
    last_y: f32,
    tiles: &TileMap,
    mut f: impl FnMut(TileCollisionEvent, usize, usize, &TileKind),
) {
    // Calculate (x1..x2).
    let x1 = last_x as usize / TILE_SIZE;
    let x2 = ((last_x + bounds.width) / TILE_SIZE as f32).ceil() as usize;

    // Calculate (y1..y2) based on distance moved.
    let (y1, y2) = if bounds.y > last_y {
        let y1 = ((last_y + bounds.height) / TILE_SIZE as f32).ceil() as usize;
        let y2 = ((bounds.y + bounds.height) / TILE_SIZE as f32).ceil() as usize;
        (y1, y2)
    } else {
        let y1 = bounds.y as usize / TILE_SIZE;
        let y2 = last_y as usize / TILE_SIZE;
        (y1, y2)
    };

    // Iterate all newly touched tiles.
    for y in y1..y2 {
        for x in x1..x2 {
            let tile = tiles[(x, y)];
            let property = TILE_PHYSICS_PROPERTIES[tile as usize]; // TODO pass this in?

            // Solid.
            if property.solid {
                // Floor hit.
                if bounds.y > last_y {
                    f(TileCollisionEvent::Floor, x, y, &tile);
                }
                // Ceiling hit.
                if bounds.y < last_y {
                    f(TileCollisionEvent::Ceiling, x, y, &tile);
                }
            }
        }
    }
}

/*pub fn resolve_generic_tile_collision_y(
    mut bounds: Aabb,
    last_y: f32,
    tiles: &TileMap,
) -> (Aabb, TileCollisionFlags) {
    // Calculate (x1..x2).
    let x1 = bounds.x as usize / TILE_SIZE;
    let x2 = ((bounds.x + bounds.width) / TILE_SIZE as f32).ceil() as usize;

    // Calculate (y1..y2) based on distance moved.
    let (y1, y2) = if bounds.y > last_y {
        let y1 = ((last_y + bounds.height) / TILE_SIZE as f32).ceil() as usize;
        let y2 = ((bounds.y + bounds.height) / TILE_SIZE as f32).ceil() as usize;
        (y1, y2)
    } else {
        let y1 = bounds.y as usize / TILE_SIZE;
        let y2 = last_y as usize / TILE_SIZE;
        (y1, y2)
    };

    // Iterate all newly touched tiles.
    let mut flags = tile_collision_flags::NONE;
    for y in y1..y2 {
        for x in x1..x2 {
            let tile = tiles[(x, y)];
            let property = TILE_PHYSICS_PROPERTIES[tile as usize]; // TODO pass this in?

            // Solid.
            if property.solid {
                if bounds.y > last_y {
                    flags |= tile_collision_flags::HIT_WALL;
                    bounds.y = (y * TILE_SIZE) as f32 - bounds.height;
                    //bounds.dy = 0.;
                }
                if bounds.y < last_y {
                    flags |= tile_collision_flags::HIT_FLOOR;
                    bounds.y = ((y + 1) * TILE_SIZE) as f32;
                    //bounds.dy *= 0.50;
                }
            }
        }
    }

    (bounds, flags)
}*/

/*
pub fn resolve_generic_tile_collision_x(
    physics: &mut GenericPhysics,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    // Calculate (x1..x2) based on distance moved.
    let (x1, x2) = if physics.x > physics.last_x {
        let x1 = ((physics.last_x + physics.width) / TILE_SIZE as f32).ceil() as usize;
        let x2 = ((physics.x + physics.width) / TILE_SIZE as f32).ceil() as usize;
        (x1, x2)
    } else {
        let x1 = physics.x as usize / TILE_SIZE;
        let x2 = physics.last_x as usize / TILE_SIZE;
        (x1, x2)
    };

    // Calculate (y1..y2).
    let y1 = physics.last_y as usize / TILE_SIZE;
    let y2 = ((physics.last_y + physics.height) / TILE_SIZE as f32).ceil() as usize;

    // Iterate all newly touched tiles.
    for y in y1..y2 {
        for x in x1..x2 {
            let src_index = x + y * stride;
            let tile = tiles[src_index];
            let property = TILE_PHYSICS_PROPERTIES[tile as usize]; // TODO pass this in?

            // Solid.
            if property.solid {
                if physics.x > physics.last_x {
                    physics.x = (x * TILE_SIZE) as f32 - physics.width;
                }
                if physics.x < physics.last_x {
                    physics.x = ((x + 1) * TILE_SIZE) as f32;
                }
                physics.dx = 0.;
            }
        }
    }
}

pub fn resolve_generic_tile_collision_y(
    physics: &mut GenericPhysics,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    // Calculate (x1..x2).
    let x1 = physics.last_x as usize / TILE_SIZE;
    let x2 = ((physics.last_x + physics.width) / TILE_SIZE as f32).ceil() as usize;

    // Calculate (y1..y2) based on distance moved.
    let (y1, y2) = if physics.y > physics.last_y {
        let y1 = ((physics.last_y + physics.height) / TILE_SIZE as f32).ceil() as usize;
        let y2 = ((physics.y + physics.height) / TILE_SIZE as f32).ceil() as usize;
        (y1, y2)
    } else {
        let y1 = physics.y as usize / TILE_SIZE;
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
                if physics.y > physics.last_y {
                    physics.flags |= HUMANOID_ON_GROUND_BIT;
                    physics.y = (y * TILE_SIZE) as f32 - physics.height;
                    physics.dy = 0.;
                }
                if physics.y < physics.last_y {
                    physics.y = ((y + 1) * TILE_SIZE) as f32;
                    physics.dy *= 0.50;
                }
            }
        }
    }
}

*/
