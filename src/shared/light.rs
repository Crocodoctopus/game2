pub const LIGHT_MAX: u8 = 40;
pub const FADE_MIN: u8 = 1;
pub const FADE_SOLID: u8 = 8;
pub const FADE_DENSE: u8 = 12;
//  [E][E] [E][E]
//  [E][0] [1][2]
//  [E][0] [0][1]
//        /------
//  [E][1]|[0][0]
//  [E][2]|[1][0]
//  [E][1]|[0][0]

pub fn create_light_map_base(w: usize, h: usize) -> Box<[u8]> {
    let mut light_map = vec![0; w * h].into_boxed_slice();

    // Create a border around the light map.
    #[rustfmt::skip]
    let _ = for x in 0..w {
        light_map[x + (0)     * w] = LIGHT_MAX;
        light_map[x + (h - 1) * w] = LIGHT_MAX;
    };
    #[rustfmt::skip]
    let _ = for y in 0..h {
        light_map[(0) +     y * w] = LIGHT_MAX;
        light_map[(w - 1) + y * w] = LIGHT_MAX;
    };

    return light_map;
}

pub fn create_fade_map_base(w: usize, h: usize) -> Box<[u8]> {
    vec![FADE_MIN; w * h].into_boxed_slice()
}

#[inline(always)]
pub fn fill_light_map(
    stride: usize,
    light_map: &mut Box<[u8]>,
    fade_map: &Box<[u8]>,
    mut probes: Vec<u16>,
) {
    assert!(light_map.len() == fade_map.len());
    assert!(light_map.len() > 4);
    assert!(stride > 2);

    // first loop through original probes.
    let mut i = 0;
    while i < probes.len() {
        let index = probes[i] as usize;
        i += 1;
        assert!(index > stride);
        assert!(index + stride < light_map.len());
        let brightness = light_map[index];
        let fade = fade_map[index];
        let new_brightness = brightness.saturating_sub(fade);

        let offsets = [index - 1, index + 1, index - stride, index + stride];
        for offset in offsets {
            if light_map[offset] < new_brightness {
                light_map[offset] = new_brightness;
                probes.push(offset as u16);
            }
        }
        /*unsafe {
            let index = *probes.get_unchecked(i) as usize;
            i += 1;
            //assert!(index > stride);
            //assert!(index + stride < light_map.len());
            let brightness = *light_map.get_unchecked(index);
            let fade = *fade_map.get_unchecked(index);
            let new_brightness = brightness.saturating_sub(fade);

            let offsets = [index - 1, index + 1, index - stride, index + stride];
            for offset in offsets {
                if *light_map.get_unchecked(offset) < new_brightness {
                    *light_map.get_unchecked_mut(offset) = new_brightness;
                    probes.push(offset as u16);
                }
            }
        }*/
    }
}
