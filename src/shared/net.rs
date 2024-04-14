use crate::shared::{Tile, CHUNK_AREA};

#[derive(Clone, bitcode::Encode, bitcode::Decode, Debug)]
pub enum ClientNetMessage {
    Connect { version: (u8, u8) },

    Join,

    RequestChunk { x: u16, y: u16, seq: u64 },
}

#[derive(Clone, bitcode::Encode, bitcode::Decode, Debug)]
pub enum ServerNetMessage {
    ConnectAccept,

    ConnectReject {
        version: (u8, u8),
    },

    WorldInfo {
        width: u16,
        height: u16,
    },

    ChunkSync {
        x: u16,
        y: u16,
        seq: u64,
        fg_tiles: [Tile; CHUNK_AREA],
        bg_tiles: [Tile; CHUNK_AREA],
    },

    Start,
}

pub fn serialize(msgs: &[impl bitcode::Encode]) -> Box<[u8]> {
    bitcode::encode(msgs).into_boxed_slice()
}

pub fn deserialize<T: bitcode::DecodeOwned>(bytes: Box<[u8]>) -> Box<[T]> {
    bitcode::decode(&bytes).unwrap_or_default()
}
