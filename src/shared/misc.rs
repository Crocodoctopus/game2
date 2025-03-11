use bitcode::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Aabb {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
