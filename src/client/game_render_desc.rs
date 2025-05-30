use crate::shared::item::ItemKind;
use crate::shared::light::Brightness;
use crate::shared::tile::TileKind;

#[derive(Copy, Clone, Debug)]
pub struct ItemRenderDesc {
    pub x: f32,
    pub y: f32,
    pub kind: ItemKind,
}

#[derive(Copy, Clone, Debug)]
pub struct SpriteRenderDesc {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct TileRenderDesc(pub TileKind);

#[derive(Clone, Debug)]
pub struct GameRenderDesc {
    // Viewport.
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,

    // Item data.
    pub items: Box<[ItemRenderDesc]>,

    // Sprite data.
    pub sprites: Box<[SpriteRenderDesc]>,

    // Light data.
    pub light_x: usize,
    pub light_y: usize,
    pub light_w: usize,
    pub light_h: usize,
    pub r_channel: Box<[Brightness]>,
    pub g_channel: Box<[Brightness]>,
    pub b_channel: Box<[Brightness]>,

    // Tile data.
    pub tiles_x: usize,
    pub tiles_y: usize,
    pub tiles_w: usize,
    pub tiles_h: usize,
    pub fg_tiles: Box<[TileRenderDesc]>,
    pub bg_tiles: Box<[TileRenderDesc]>,
}
