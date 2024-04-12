use crate::tile::*;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum HumanoidFlags {
    OnGround = 1 << 1,
}

#[derive(Clone, Debug, Default)]
pub struct Humanoid {
    pub id: usize,

    // Dimensions.
    pub w: f32,
    pub h: f32,

    // Physics.
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub ddx: f32,
    pub ddy: f32,
    pub flags: u8,
}

pub fn update_humanoid_physics_x(humanoid: &mut Humanoid, ft: f32) {
    humanoid.x += 0.5 * humanoid.ddx * ft * ft + humanoid.dx * ft;
    humanoid.dx += humanoid.ddx * ft;
}

pub fn resolve_humanoid_tile_collision_x(
    humanoid: &mut Humanoid,
    last_x: f32,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    // Calculate (x1..x2) based on distance moved.
    let (x1, x2) = if humanoid.x > last_x {
        let x1 = ((last_x + humanoid.w) / TILE_SIZE as f32).ceil() as usize;
        let x2 = ((humanoid.x + humanoid.w) / TILE_SIZE as f32).ceil() as usize;
        (x1, x2)
    } else {
        let x1 = humanoid.x as usize / TILE_SIZE;
        let x2 = last_x as usize / TILE_SIZE;
        (x1, x2)
    };

    // Calculate (y1..y2).
    let y1 = humanoid.y as usize / TILE_SIZE;
    let y2 = ((humanoid.y + humanoid.h) / TILE_SIZE as f32).ceil() as usize;

    // Iterate all newly touched tiles.
    for y in y1..y2 {
        for x in x1..x2 {
            let src_index = x + y * stride;
            let tile = tiles[src_index];
            let property = TILE_PHYSICS_PROPERTIES[tile as usize]; // TODO pass this in?

            // Solid.
            if property.solid {
                if humanoid.x > last_x {
                    humanoid.x = (x * TILE_SIZE) as f32 - humanoid.w;
                }
                if humanoid.x < last_x {
                    humanoid.x = ((x + 1) * TILE_SIZE) as f32;
                }
                humanoid.dx = 0.;
            }
        }
    }
}

pub fn update_humanoid_physics_y(humanoid: &mut Humanoid, ft: f32) {
    humanoid.y += 0.5 * humanoid.ddy * ft * ft + humanoid.dy * ft;
    humanoid.dy += humanoid.ddy * ft;
}

pub fn resolve_humanoid_tile_collision_y(
    humanoid: &mut Humanoid,
    last_y: f32,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    // Calculate (x1..x2).
    let x1 = humanoid.x as usize / TILE_SIZE;
    let x2 = ((humanoid.x + humanoid.w) / TILE_SIZE as f32).ceil() as usize;

    // Calculate (y1..y2) based on distance moved.
    let (y1, y2) = if humanoid.y > last_y {
        let y1 = ((last_y + humanoid.h) / TILE_SIZE as f32).ceil() as usize;
        let y2 = ((humanoid.y + humanoid.h) / TILE_SIZE as f32).ceil() as usize;
        (y1, y2)
    } else {
        let y1 = humanoid.y as usize / TILE_SIZE;
        let y2 = last_y as usize / TILE_SIZE;
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
                if humanoid.y > last_y {
                    humanoid.flags |= HumanoidFlags::OnGround as u8;
                    humanoid.y = (y * TILE_SIZE) as f32 - humanoid.h;
                    humanoid.dy = 0.;
                }
                if humanoid.y < last_y {
                    humanoid.y = ((y + 1) * TILE_SIZE) as f32;
                    humanoid.dy *= 0.50;
                }
            }
        }
    }
}
