use crate::shared::tile::*;
use std::collections::HashMap;

pub struct TileDamage {
    timestamp: u8, // Time last touched, in 100 milliseconds
    hp: u8,
}

pub fn register_tile_hit(
    tile_damages: &mut HashMap<u32, TileDamage>,
    index: u32,
    tile: Tile,
    timestamp: u64, // in us
) {
    // Skip None.
    if matches!(tile, Tile::None) {
        return;
    }

    // Insert a TileDamage associated with tile_index.
    let tile_damage = tile_damages.entry(index).or_insert(TileDamage {
        timestamp: 0,
        hp: 200,
    });

    // Pretend some properties for now
    tile_damage.timestamp = (timestamp / 1000 / 100) as u8;
    tile_damage.hp = tile_damage.hp.saturating_sub(50);
}

pub fn remove_tile_damage(tile_damages: &mut HashMap<u32, TileDamage>, index: u32) {
    tile_damages.remove(&index);
}

pub fn update_tile_damages(
    tile_damages: &mut HashMap<u32, TileDamage>,
    timestamp: u64,
) -> Vec<u32> {
    // Collect all dead tiles.
    let tiles_destroyed = tile_damages
        .iter()
        .filter(|(_, tile_damage)| tile_damage.hp == 0)
        .map(|(index, _)| *index)
        .collect();

    let timestamp = (timestamp / 1000 / 100) as u8;
    tile_damages.retain(|_, tile_damage| {
        // If the last time this TileDamage was touched was 7s ago.
        if timestamp.wrapping_sub(tile_damage.timestamp) as i8 > 70 {
            tile_damage.hp = tile_damage.hp.saturating_add(50); // Add 25% hp.
            tile_damage.timestamp = timestamp.wrapping_sub(25); // A bit of a hack.
        }

        // Retain only tiles that aren't fully dead or fully alive.
        tile_damage.hp > 0 && tile_damage.hp < 200
    });

    tiles_destroyed
}
