use noise::{NoiseFn, Perlin};
use std::collections::BinaryHeap;
use std::cmp::Reverse;

const SEA_LEVEL: f32 = 0.7;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Block {
    Air,
    Water,
    Dirt,
    Rock,
    Sand,
    Coal,
    Iron,
    Tin,
    Copper,
}

impl Block {
    fn to_u8(self) -> u8 {
        match self {
            Block::Air => 0,
            Block::Water => 0,
            Block::Dirt => 1,
            Block::Rock => 2,
            Block::Sand => 1,
            Block::Coal => 0,
            Block::Iron => 1,
            Block::Tin => 2,
            Block::Copper => 0,
        }
    }
}

pub fn generate_terrain(width: usize, height: usize, seed: u32) -> Vec<u8> {
    let mut world = vec![vec![Block::Air; width]; height];

    for i in 0..((SEA_LEVEL * height as f32) as usize) {
        for j in 0..width {
            world[i][j] = Block::Water;
        }
    }

    let perlin = Perlin::new(seed);

    //let mut water_heap: BinaryHeap<Reverse<(usize, usize, Option<bool>)>> = BinaryHeap::new();
    let mut water_heap: BinaryHeap<Reverse<(usize, usize, Option<bool>)>> = BinaryHeap::new();

    let base_sealevel = (height as f32 * SEA_LEVEL) as usize;
    // generate hill tops and mountain tops between 0.4 and 0.6 and fill in the space below
    //for i in ((height as f32 * 0.4) as usize)..((height as f32 * 0.6) as usize) {
    for j in 0..width {
        let x = j as f64 / width as f64;
        let z = 0.0;

        let noise_greater = perlin.get([x * 7., 321., z]) as f32;
        let noise_greater = noise_greater / 10.;
        let noise = perlin.get([x * 14., 123., z]) as f32;
        let noise = (noise / 10.) * 1.2;

        let base1 = (height as f32 * (SEA_LEVEL + noise)) as usize;
        let base2 = (height as f32 * (SEA_LEVEL + noise_greater)) as usize;
        let base = std::cmp::max(base1, base2);

        for i in 0..base {
            let y = i as f64 / height as f64;


            if i < base_sealevel {
                let lake = perlin.get([x * 13., y * 26., z + 123.]) as f32;
                if lake > 0.45 {
                    water_heap.push(Reverse((i, j, None)));
                    world[i][j] = Block::Water;
                    continue;
                }
            }

            /*
            // old cave generation strands
            let adj_amt = 0.08;
            let cave_noise_left = perlin.get([x * 60. - adj_amt, y * 75. - adj_amt, z]) as f32;
            let cave_noise_mid = perlin.get([x * 60., y * 75., z]) as f32;
            let cave_noise_right = perlin.get([x * 60. + adj_amt, y * 75. + adj_amt, z]) as f32;
            let is_max = cave_noise_left < cave_noise_mid && cave_noise_mid > cave_noise_right;
            let is_min = cave_noise_left > cave_noise_mid && cave_noise_mid < cave_noise_right;
            if cave_noise_mid > (0.6 - (y as f32) * 2. ) && (is_max || is_min) {
                world[i][j] = Block::Air;
                continue;
            }
            */

            // cave mask smoothly limits cave locations
            //let cave_mask = perlin.get([x * -10., y * -7., z + 3.]) as f32;

            //if cave_mask + y as f32 / 4. > 0.2 {
                let cave_noise_mid = perlin.get([x * 30., y * 45., z]) as f32;
                //if cave_noise_mid < (0.1 * y as f32) && cave_noise_mid > (-0.1 * y as f32) {
                if cave_noise_mid < 0.1 && cave_noise_mid > -0.1 {
                    world[i][j] = Block::Air;
                    continue;
                }
            //}

            if i > (base - 12) {
                if base < base_sealevel {
                    world[i][j] = Block::Sand;
                } else {
                    world[i][j] = Block::Dirt;
                }
                continue;
            }

            let noise = perlin.get([x * 100., y * 100., z]) as f32;
            let noise = (noise + 1.0) / 2.;
            if noise < 0.1 {
                world[i][j] = Block::Air;
            } else if noise < 0.15 {
                world[i][j] = Block::Sand;
            } else if noise < 0.25 {
                world[i][j] = Block::Dirt;
            } else if noise < 0.75 {
                world[i][j] = Block::Rock;
            } else {
                if y as f32 > 0.2 {
                    world[i][j] = Block::Iron;
                } else if y as f32 > 0.4 {
                    world[i][j] = Block::Tin;
                } else if y as f32 > 0.6 {
                    world[i][j] = Block::Copper;
                } else {
                    world[i][j] = Block::Coal;
                }
                /*
                let noise = perlin.get([x * 123., y * 123., z]) as f32;
                let noise = (noise + 1.0) / 2.;
                if noise < (0.65 - y as f32) {
                    world[i][j] = Block::Iron;
                } else if noise < (0.75 - y as f32) {
                    world[i][j] = Block::Tin;
                } else if noise < (0.9 - y as f32) {
                    world[i][j] = Block::Copper;
                } else {
                    world[i][j] = Block::Coal;
                }
                */
            }
        }
    }
    //}

    //settle_water(&mut world, &mut water_heap);
    let mut out = vec![0; width * height];

    for i in 5..height {
        for j in 0..width {
            let val = world[i][j].to_u8();
            out[i * width + j] = val;
        }
    }

    out
}

// heap is stored in y, x order
fn settle_water(world: &mut Vec<Vec<Block>>, water_heap: &mut BinaryHeap<Reverse<(usize, usize, Option<bool>)>>) {
    let width = world.len();
    let height = world[0].len();
    for _ in 0..1600 {
        let mut iter_heap = BinaryHeap::new();
        let mut literally_anything_happened = false;
        while let Some(Reverse((y, x, going_left))) = water_heap.pop() {
            if y > 0 && world[y - 1][x] == Block::Air {
                world[y - 1][x] = Block::Water;
                world[y][x] = Block::Air;
                iter_heap.push(Reverse((y - 1, x, None)));
                literally_anything_happened = true;
                continue;
            }
            if x > 0 && world[y][x - 1] == Block::Air && matches!(going_left, None | Some(true)) {
                world[y][x - 1] = Block::Water;
                world[y][x] = Block::Air;
                iter_heap.push(Reverse((y, x - 1, Some(true))));
                literally_anything_happened = true;
                continue;
            }
            if x < width - 1 && world[y][x + 1] == Block::Air && matches!(going_left, None | Some(false)) {
                world[y][x + 1] = Block::Water;
                world[y][x] = Block::Air;
                iter_heap.push(Reverse((y, x + 1, Some(false))));
                literally_anything_happened = true;
                continue;
            }
            iter_heap.push(Reverse((y, x, None)));
        }
        if !literally_anything_happened {
            break;
        }
        *water_heap = iter_heap;
    }
}