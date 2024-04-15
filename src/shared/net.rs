use crate::shared::{Humanoid, HumanoidId, Tile, CHUNK_AREA};
use bitcode::{decode, encode, Decode, DecodeOwned, Encode};
use std::collections::HashMap;

pub trait NetMessage: Encode + DecodeOwned {}
impl NetMessage for ClientNetMessage {}
impl NetMessage for ServerNetMessage {}

#[derive(Clone, Encode, Decode, Debug)]
pub enum ClientNetMessage {
    Ping,

    Connect { version: (u8, u8) },

    Join,

    JoinComplete,

    SyncPlayer { player: Humanoid },

    RequestChunk { x: u16, y: u16, seq: u32 },
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
        id: HumanoidId,
        spawn_x: u16,
        spawn_y: u16,
    },

    ChunkSync {
        x: u16,
        y: u16,
        seq: u32,
        fg_tiles: [Tile; CHUNK_AREA],
        bg_tiles: [Tile; CHUNK_AREA],
    },

    HumanoidSync {
        humanoids: HashMap<HumanoidId, Humanoid>,
    },

    Start,
}

pub fn serialize(msgs: &[impl NetMessage]) -> Box<[u8]> {
    encode(msgs).into_boxed_slice()
}

pub fn deserialize<T: NetMessage>(bytes: Box<[u8]>) -> Box<[T]> {
    decode(&bytes).unwrap_or_default()
}
