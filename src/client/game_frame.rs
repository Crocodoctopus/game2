use crate::shared::Tile;

#[derive(Clone, Debug)]
pub struct GameFrame {
    // Viewport.
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,

    // Light data.
    pub light_x: usize,
    pub light_y: usize,
    pub light_w: usize,
    pub light_h: usize,
    pub r_channel: Box<[u8]>,
    pub g_channel: Box<[u8]>,
    pub b_channel: Box<[u8]>,

    // Tile data.
    pub tiles_x: usize,
    pub tiles_y: usize,
    pub tiles_w: usize,
    pub tiles_h: usize,
    pub fg_tiles: Box<[Tile]>,
    pub bg_tiles: Box<[Tile]>,
}
