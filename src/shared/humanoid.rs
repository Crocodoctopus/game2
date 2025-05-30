use crate::shared::misc::Aabb;
use crate::shared::physics::*;
use crate::shared::tile::*;
use crate::shared::tile_collision::*;
use crate::shared::GlobalId;
use bitcode::{Decode, Encode};
use std::collections::HashMap;

#[derive(Clone, Debug, Encode, Decode)]
pub struct Humanoid {
    pub bounds: Aabb,
    pub last_x: f32,
    pub last_y: f32,
    pub flags: TileCollisionFlags,
    pub input: HumanoidInput,
    pub physics: GenericPhysics,
    pub ai: HumanoidAi,
    //pub entity_collider: ColliderHandle,
}

pub fn update_humanoid_ais(
    humanoids: &mut HashMap<GlobalId, Humanoid>,
    //col_sys: &CollisionSystem,
    tiles: &TileMap,
) {
    // Clone all players.
    let players: HashMap<GlobalId, Humanoid> = humanoids
        .iter()
        .filter(|(_, humanoid)| matches!(humanoid.ai, HumanoidAi::Player))
        .map(|(&id, humanoid)| (id, humanoid.clone()))
        .collect();

    for humanoid in humanoids.values_mut() {
        match humanoid.ai {
            // Ignore players.
            HumanoidAi::Player => {}

            //
            HumanoidAi::Zombie => {
                // Get closest target.
                let mut distance = f32::INFINITY;
                let mut target = None;
                for (id, player) in &players {
                    let dx = player.bounds.x - humanoid.bounds.x;
                    let dy = player.bounds.y - humanoid.bounds.y;
                    let rr = dx * dx + dy * dy;
                    if rr < distance {
                        distance = rr;
                        target = Some(id);
                    }
                }

                //
                if let Some(player) = target.and_then(|id| players.get(id)) {
                    // Move towards target.
                    let move_right = player.bounds.x > humanoid.bounds.x;
                    let move_left = player.bounds.x < humanoid.bounds.x;
                    if move_right {
                        humanoid.input.right_queue |= 1;
                    }
                    if move_left {
                        humanoid.input.left_queue |= 1;
                    }

                    // Jump over pits if target is above.
                    if (move_left || move_right) && player.bounds.y <= humanoid.bounds.y {
                        let x = if move_left {
                            humanoid.bounds.x as usize / TILE_SIZE
                        } else {
                            (humanoid.bounds.x + humanoid.bounds.width) as usize / TILE_SIZE - 1
                        };
                        let y = (humanoid.bounds.y + humanoid.bounds.height) as usize / TILE_SIZE;
                        let t0 = tiles[(x, y)];
                        let t1 = tiles[(x + 1, y)];
                        if matches!(t0, TileKind::None)
                            && matches!(t1, TileKind::None)
                            && (humanoid.flags & tile_collision_flags::HIT_FLOOR > 0)
                        {
                            humanoid.input.jump_queue |= 1;
                        }
                    }

                    // Jump if wall.
                    if move_left || move_right {
                        let x = if move_left {
                            (humanoid.bounds.x - 1.) as usize / TILE_SIZE
                        } else {
                            (humanoid.bounds.x + humanoid.bounds.width + 1.) as usize / TILE_SIZE
                        };
                        let y =
                            (humanoid.bounds.y + humanoid.bounds.height - 1.) as usize / TILE_SIZE;
                        let t0 = tiles[(x, y)];
                        let t1 = TileKind::Dirt; // tiles[x + (y - 1) * stride];
                        if !matches!(t0, TileKind::None)
                            && !matches!(t1, TileKind::None)
                            && (humanoid.flags & tile_collision_flags::HIT_FLOOR > 0)
                        {
                            humanoid.input.jump_queue |= 1;
                        }
                    }
                }
            }
        }
    }
}

pub fn update_humanoid_inputs(humanoids: &mut HashMap<GlobalId, Humanoid>) {
    for humanoid in humanoids.values_mut() {
        let max_dx = 150f32;
        if humanoid.input.right_queue & 1 != 0 && humanoid.physics.dx < max_dx {
            humanoid.physics.ddx += 1500.;
        } else if humanoid.input.left_queue & 1 != 0 && humanoid.physics.dx > -max_dx {
            humanoid.physics.ddx -= 1500.;
        } else {
            #[allow(clippy::collapsible_else_if)]
            if humanoid.physics.dx.abs() > 5. {
                humanoid.physics.ddx = -humanoid.physics.dx.signum() * 500.;
            } else {
                humanoid.physics.dx = 0.;
            }
        }

        // Check if jump was pressed at all during the last 3 frames.
        let jump_buffer = (0..3)
            .map(|i| humanoid.input.jump_queue >> i & 0b11 == 0b01)
            .reduce(|b, acc| acc | b)
            .unwrap();

        if jump_buffer && (humanoid.flags & tile_collision_flags::HIT_FLOOR > 0) {
            humanoid.physics.dy -= 300.;
        }

        // Advance input.
        humanoid.input.right_queue <<= 1;
        humanoid.input.left_queue <<= 1;
        humanoid.input.jump_queue <<= 1;
    }
}

pub fn update_humanoid_physics(
    humanoids: &mut HashMap<GlobalId, Humanoid>,
    ft: f32,
    tiles: &TileMap,
) {
    for humanoid in humanoids.values_mut() {
        // X physics.
        humanoid.last_x = humanoid.bounds.x;
        update_generic_physics_x(&mut humanoid.bounds.x, &mut humanoid.physics, ft);
        humanoid.physics.ddx = 0.;

        // Y physics.
        humanoid.physics.ddy += 500.;
        humanoid.last_y = humanoid.bounds.y;
        update_generic_physics_y(&mut humanoid.bounds.y, &mut humanoid.physics, ft);
        humanoid.physics.ddy = 0.;

        // X tile collision.
        resolve_generic_tile_collision_x(
            humanoid.bounds,
            humanoid.last_x,
            humanoid.last_y,
            tiles,
            |event, x, _y, _tile| match event {
                TileCollisionEvent::LeftWall => {
                    humanoid.bounds.x = ((x + 1) * TILE_SIZE) as f32;
                    humanoid.physics.dx = 0.;
                }
                TileCollisionEvent::RightWall => {
                    humanoid.bounds.x = (x * TILE_SIZE) as f32 - humanoid.bounds.width;
                    humanoid.physics.dx = 0.;
                }
                TileCollisionEvent::Ceiling | TileCollisionEvent::Floor => unreachable!(),
            },
        );

        // Y tile collision.
        humanoid.flags &= !tile_collision_flags::HIT_FLOOR;
        resolve_generic_tile_collision_y(
            humanoid.bounds,
            humanoid.last_x,
            humanoid.last_y,
            tiles,
            |event, _x, y, _tile| match event {
                TileCollisionEvent::Floor => {
                    humanoid.bounds.y = (y * TILE_SIZE) as f32 - humanoid.bounds.height;
                    humanoid.physics.dy = 0.;
                    humanoid.flags |= tile_collision_flags::HIT_FLOOR;
                }
                TileCollisionEvent::Ceiling => {
                    humanoid.bounds.y = ((y + 1) * TILE_SIZE) as f32;
                    humanoid.physics.dy *= 0.5;
                    humanoid.flags |= tile_collision_flags::HIT_CEILING;
                }
                TileCollisionEvent::LeftWall | TileCollisionEvent::RightWall => unreachable!(),
            },
        );
    }
}

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
    pub max_dx: f32,
    pub dx: f32,
    pub dy: f32,
    pub ddx: f32,
    pub ddy: f32,
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum HumanoidAi {
    Player,
    Zombie,
}

pub struct HumanoidAnimation {}
