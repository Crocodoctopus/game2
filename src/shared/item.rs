use super::misc::Aabb;
use crate::shared::physics::GenericPhysics;
use bitcode::{Decode, Encode};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, Encode, Decode, Hash)]
pub struct TileId(u32);

pub struct Item {
    bounds: Aabb,
    physics: GenericPhysics,
}
