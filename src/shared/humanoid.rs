use crate::shared::{Tile, TILE_PHYSICS_PROPERTIES, TILE_SIZE};
use bitcode::{Decode, Encode};
use std::collections::HashMap;

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

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct Humanoids {
    pub index_map: HashMap<HumanoidId, usize>,
    pub humanoid_ids: Vec<HumanoidId>,
    pub humanoids: Vec<Humanoid>,
}

impl Humanoids {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn insert(&mut self, id: HumanoidId, humanoid: Humanoid) {
        let index = self.humanoid_ids.len();
        self.index_map.insert(id, index);
        self.humanoid_ids.push(id);
        self.humanoids.push(humanoid);
    }

    pub fn remove(&mut self, id: HumanoidId) {
        let Some(index) = self.index_map.remove(&id) else {
            // Id not in container.
            return;
        };

        self.humanoid_ids.swap_remove(index);
        self.humanoids.swap_remove(index);

        // Correct the index_map.
        self.humanoid_ids
            .get(index as usize)
            .and_then(|id| self.index_map.get_mut(id))
            .map(|ix| *ix = index);
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Humanoid {
    pub base: HumanoidBase,
    pub ai: HumanoidAi,
    pub input: HumanoidInput,
    pub physics: HumanoidPhysics,
}

pub fn update_humanoid_ais(
    humanoids: &mut HashMap<HumanoidId, Humanoid>,
    //col_sys: &CollisionSystem,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    // Clone bases because #rust
    let cpy: HashMap<HumanoidId, (HumanoidBase, HumanoidAi)> = humanoids
        .iter()
        .map(|(id, humanoids)| (*id, (humanoids.base.clone(), humanoids.ai.clone())))
        .collect();
    for Humanoid {
        base, ai, input, ..
    } in humanoids.values_mut()
    {
        match ai {
            // Player is special.
            HumanoidAi::Player => {}

            //
            HumanoidAi::Zombie { mut target } => {
                // Update target.
                {
                    // Get all targets in range.
                    let mut distance = f32::INFINITY;
                    target = None;
                    for (id, (base2, ai2)) in &cpy {
                        if matches!(ai2, HumanoidAi::Player) {
                            let dx = base2.x - base.x;
                            let dy = base2.y - base.y;
                            let rr = dx * dx + dy * dy;
                            if rr < distance {
                                distance = rr;
                                target = Some(*id);
                            }
                        }
                    }
                    //let targets = col_sys.get_collisions(detect_range);

                    //
                    /*if !targets.cotains(target) {
                        target = targets.first();
                    }*/
                }

                match target {
                    // Seek.
                    Some(target) => {
                        // Move towards target.
                        let target_base = &cpy[&target].0;
                        if target_base.x > base.x {
                            input.right_queue |= 1;
                        }
                        if target_base.x < base.x {
                            input.left_queue |= 1;
                        }

                        // Jump is over a pit and target is higher in elevation.

                        // If a wall blocks way, jump.

                        // If stuck (x has not changed for N frames) turn around and force roam for M frames.
                    }

                    // Roam
                    None => {
                        // More forward.

                        // Jump over all pits.

                        // If a wall blocks way, jump.

                        // If stuck (x has not change dfor N frames) turn around.
                    }
                }
            }
        }
    }
}

pub fn update_humanoid_inputs(humanoids: &mut HashMap<HumanoidId, Humanoid>) {
    for Humanoid {
        ref mut base,
        ref mut physics,
        ref mut input,
        ..
    } in humanoids.values_mut()
    {
        if input.right_queue & 1 != 0 && physics.dx < 150. {
            physics.ddx += 1500.;
        } else if input.left_queue & 1 != 0 && physics.dx > -150. {
            physics.ddx -= 1500.;
        } else {
            physics.ddx = -physics.dx.signum() * 500.;
        }

        // Check if jump was pressed at all during the last 3 frames.
        let jump_buffer = (0..3)
            .into_iter()
            .map(|i| input.jump_queue >> i & 0b11 == 0b01)
            .reduce(|b, acc| acc | b)
            .unwrap();

        if jump_buffer && base.flags & HUMANOID_ON_GROUND_BIT != 0 {
            physics.dy -= 300.;
        }

        // Advance input.
        input.right_queue <<= 1;
        input.left_queue <<= 1;
        input.jump_queue <<= 1;
    }
}

pub fn update_humanoid_physics(humanoids: &mut HashMap<HumanoidId, Humanoid>, ft: f32) {
    for Humanoid {
        ref mut base,
        ref mut physics,
        ..
    } in humanoids.values_mut()
    {
        // Gravity.
        physics.ddy += 500.;

        update_humanoid_physics_y(base, physics, ft);
        physics.ddy = 0.;

        update_humanoid_physics_x(base, physics, ft);
        physics.ddx = 0.;
    }
}

pub fn resolve_humanoid_tile_collisions(
    humanoids: &mut HashMap<HumanoidId, Humanoid>,
    stride: usize,
    tiles: &Box<[Tile]>,
) {
    for Humanoid {
        ref mut base,
        ref mut physics,
        ..
    } in humanoids.values_mut()
    {
        base.flags &= !HUMANOID_ON_GROUND_BIT; 
        resolve_humanoid_tile_collision_x(base, physics, stride, tiles);
        resolve_humanoid_tile_collision_y(base, physics, stride, tiles);
    }
}

pub type HumanoidFlags = u8;
pub const HUMANOID_ON_GROUND_BIT: u8 = 1 << 1;

#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct HumanoidInput {
    pub jump_queue: u8,
    pub left_queue: u8,
    pub right_queue: u8,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct HumanoidBase {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub flags: u8,
}

#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct HumanoidPhysics {
    pub last_x: f32,
    pub last_y: f32,
    pub dx: f32,
    pub dy: f32,
    pub ddx: f32,
    pub ddy: f32,
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum HumanoidAi {
    Player,
    Zombie { target: Option<HumanoidId> },
}

pub struct HumanoidAnimation {}

pub fn update_humanoid_physics_x(base: &mut HumanoidBase, physics: &mut HumanoidPhysics, ft: f32) {
    physics.last_x = base.x;
    base.x += 0.5 * physics.ddx * ft * ft + physics.dx * ft;
    physics.dx += physics.ddx * ft;
}

#[test]
fn physics_test() {
    let dt0 = 16_666 as f32 / 1e6;
    let dt1 = 33_332 as f32 / 1e6;

    let mut base0 = HumanoidBase {
        x: 0.,
        y: 0.,
        w: 0.,
        h: 0.,
        flags: 0,
    };
    let mut physics0 = HumanoidPhysics {
        dx: 15.,
        dy: 10.,
        ddx: 90.,
        ddy: 200.,
        ..Default::default()
    };

    let mut base1 = base0.clone();
    let mut physics1 = physics0.clone();

    update_humanoid_physics_x(&mut base0, &mut physics0, dt0);
    update_humanoid_physics_y(&mut base0, &mut physics0, dt0);
    update_humanoid_physics_x(&mut base0, &mut physics0, dt0);
    update_humanoid_physics_y(&mut base0, &mut physics0, dt0);
    
    update_humanoid_physics_x(&mut base1, &mut physics1, dt1);
    update_humanoid_physics_y(&mut base1, &mut physics1, dt1);
    
    assert_eq!(base0.x, base1.x);
    assert_eq!(base0.y, base1.y);
    assert_eq!(physics0.dx, physics1.dx);
    assert_eq!(physics0.dy, physics1.dx);
    assert_eq!(physics0.ddx, physics1.ddx);
    assert_eq!(physics0.ddy, physics1.ddy);
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
