use crate::shared::humanoid::*;
use crate::shared::item::*;
use crate::shared::tile::*;
use crate::shared::GlobalId;
use bitcode::{decode, encode, Decode, DecodeOwned, Encode};

pub trait NetMessage: Encode + DecodeOwned {}
impl NetMessage for ClientNetMessage {}
impl NetMessage for ServerNetMessage {}

#[derive(Clone, Encode, Decode, Debug)]
pub struct NetItem {
    pub kind: ItemKind,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Encode, Decode, Debug)]
pub struct NetHumanoid(pub Humanoid);

#[derive(Clone, Encode, Decode, Debug)]
pub enum ClientNetMessage {
    Ping,

    Connect { version: (u8, u8) },

    Join,

    JoinComplete,

    SyncPlayer { player: Humanoid },

    HitTile { x: u16, y: u16 },

    RequestChunk { x: u16, y: u16 },
}

#[derive(Clone, Encode, Decode, Debug)]
pub enum ServerNetMessage {
    Ping,

    // Expect Join.
    ConnectAccept,

    ConnectReject {
        version: (u8, u8),
    },

    JoinAccept {
        width: u16,
        height: u16,
        id: GlobalId,
        spawn_x: u16,
        spawn_y: u16,
    },

    ChunkSync {
        x: u16,
        y: u16,
        fg_tiles: [TileKind; CHUNK_AREA],
        bg_tiles: [TileKind; CHUNK_AREA],
    },

    TileSync {
        x: u16,
        y: u16,
        tile: TileKind,
    },

    ItemSync {
        items: Box<[(GlobalId, NetItem)]>,
    },

    HumanoidSync {
        humanoids: Box<[(GlobalId, NetHumanoid)]>,
    },

    Start,
}

pub fn serialize(msgs: &[impl NetMessage]) -> Box<[u8]> {
    encode(msgs).into_boxed_slice()
}

pub fn deserialize<T: NetMessage>(bytes: Box<[u8]>) -> Box<[T]> {
    decode(&bytes).unwrap_or_default()
}
