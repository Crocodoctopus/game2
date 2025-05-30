use super::GlobalId;
use super::misc::Aabb;
use crate::shared::physics::*;
use crate::shared::tile::*;
use crate::shared::tile_collision::*;
use bitcode::{Decode, Encode};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum ItemKind {
    Tile(TileKind),
}

pub struct ItemProperty {
    u: f32,
    v: f32,
    width: f32,
    height: f32,
}

impl ItemKind {
    pub fn get_property(&self) -> ItemProperty {
        match self {
            Self::Tile(kind) => {
                let ttp = TILE_TEXTURE_PROPERTIES[*kind as usize];
                ItemProperty {
                    u: ttp.u,
                    v: ttp.v,
                    width: TILE_SIZE as f32,
                    height: TILE_SIZE as f32,
                }
            }
            _ => unimplemented!(),
        }
    }
}

pub struct Item {
    pub bounds: Aabb,
    pub last_x: f32,
    pub last_y: f32,
    pub physics: GenericPhysics,
    pub flags: TileCollisionFlags,
    pub kind: ItemKind,
    pub count: u8,
}

pub fn update_item_physics(items: &mut HashMap<GlobalId, Item>, ft: f32, tiles: &TileMap) {
    for item in items.values_mut() {
        // X physics.
        item.last_x = item.bounds.x;
        update_generic_physics_x(&mut item.bounds.x, &mut item.physics, ft);
        item.physics.ddx = 0.;

        // Apply friction.
        if item.flags & tile_collision_flags::HIT_FLOOR > 0 {
            item.physics.ddx = -item.physics.dx.signum() * 500.;
        }

        // Y physics.
        item.physics.ddy += 500.;
        item.last_y = item.bounds.y;
        update_generic_physics_y(&mut item.bounds.y, &mut item.physics, ft);
        item.physics.ddy = 0.;

        // X tile collision.
        resolve_generic_tile_collision_x(
            item.bounds,
            item.last_x,
            item.last_y,
            tiles,
            |event, x, _y, _tile| match event {
                TileCollisionEvent::LeftWall => {
                    item.bounds.x = ((x + 1) * TILE_SIZE) as f32;
                    item.physics.dx = 0.;
                }
                TileCollisionEvent::RightWall => {
                    item.bounds.x = (x * TILE_SIZE) as f32 - item.bounds.width;
                    item.physics.dx = 0.;
                }
                TileCollisionEvent::Ceiling | TileCollisionEvent::Floor => unreachable!(),
            },
        );

        // Y tile collision.
        item.flags &= !tile_collision_flags::HIT_FLOOR;
        resolve_generic_tile_collision_y(
            item.bounds,
            item.last_x,
            item.last_y,
            tiles,
            |event, _x, y, _tile| match event {
                TileCollisionEvent::Floor => {
                    item.bounds.y = (y * TILE_SIZE) as f32 - item.bounds.height;
                    item.physics.dy = 0.;
                    item.flags |= tile_collision_flags::HIT_FLOOR;
                }
                TileCollisionEvent::Ceiling => {
                    item.bounds.y = ((y + 1) * TILE_SIZE) as f32;
                    item.physics.dy *= 0.5;
                    item.flags |= tile_collision_flags::HIT_CEILING;
                }
                TileCollisionEvent::LeftWall | TileCollisionEvent::RightWall => unreachable!(),
            },
        );
    }
}
