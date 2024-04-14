pub mod humanoid;
pub mod light;
pub mod net;
pub mod tile;

pub use humanoid::*;
pub use light::*;
pub use net::*;
pub use tile::*;

// Chunk.
pub const CHUNK_SIZE: usize = 8;
pub const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;

// View.
pub const CHUNK_LOAD_WIDTH: usize = 10;
pub const CHUNK_LOAD_HEIGHT: usize = 6;
