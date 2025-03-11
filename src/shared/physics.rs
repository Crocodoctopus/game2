use bitcode::{Decode, Encode};

#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct GenericPhysics {
    pub dx: f32,
    pub dy: f32,
    pub ddx: f32,
    pub ddy: f32,
}

pub fn update_generic_physics_x(x: &mut f32, physics: &mut GenericPhysics, dt: f32) {
    *x += 0.5 * physics.ddx * dt * dt + physics.dx * dt;
    physics.dx += physics.ddx * dt;
}

pub fn update_generic_physics_y(y: &mut f32, physics: &mut GenericPhysics, dt: f32) {
    *y += 0.5 * physics.ddy * dt * dt + physics.dy * dt;
    physics.dy += physics.ddy * dt;
}
