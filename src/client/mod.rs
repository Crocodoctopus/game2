pub mod client;
mod game_frame;
#[cfg(feature = "opengl")]
mod game_render_state;
#[cfg(feature = "wgpu")]
mod game_render_state_wgpu;
mod game_update_state;

pub use client::*;
use game_frame::*;
#[cfg(feature = "opengl")]
use game_render_state::*;
#[cfg(feature = "wgpu")]
use game_render_state_wgpu::*;
use game_update_state::*;
