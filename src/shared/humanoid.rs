use crate::tile::*;

#[repr(u8)]
enum HumanoidFlags {
    OnGround = 1 << 1,
}

struct Humanoid {
    id: usize,

    // Dimensions.
    w: f32,
    h: f32,

    // Physics.
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    ddx: f32,
    ddy: f32,
    flags: u8,
}

fn update_humanoid_physics_x(humanoid: &mut Humanoid, ft: f32) {
    humanoid.x += 0.5 * humanoid.ddx * ft * ft + humanoid.dx * ft;
    humanoid.dx += humanoid.ddx * ft;
}

fn resolve_humanoid_tile_collision_x(
    humanoid: &mut Humanoid,
    last_x: f32,
    stride: usize,
    tiles: Box<[Tile]>,
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
                    humanoid.x = (x * (TILE_SIZE + 1)) as f32;
                }
                humanoid.dx = 0.;
            }
        }
    }
}
