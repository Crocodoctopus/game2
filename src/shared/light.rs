pub const LIGHT_MIN: Brightness = Brightness(0);
pub const LIGHT_MAX: Brightness = Brightness(40);
pub const FADE_MIN: Opacity = Opacity(1);
pub const FADE_SOLID: Opacity = Opacity(6);
pub const FADE_DENSE: Opacity = Opacity(12);

// Represents a valid brightness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Brightness(u8);

impl Brightness {
    pub fn new(light: u8) -> Option<Self> {
        (light <= LIGHT_MAX.0).then(|| Self(light))
    }

    pub fn raw(self) -> u8 {
        self.0
    }

    pub fn clamp(val: u8) -> Self {
        Self(LIGHT_MAX.0.clamp(0, val))
    }

    pub fn clamp3(r: u8, g: u8, b: u8) -> (Self, Self, Self) {
        (
            Brightness::clamp(r),
            Brightness::clamp(g),
            Brightness::clamp(b),
        )
    }

    pub fn apply_fade(self, fade: Opacity) -> Self {
        Self(self.0.saturating_sub(fade.0))
    }
}

// Represents a valid opacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Opacity(u8);

impl Opacity {
    pub fn new(fade: u8) -> Option<Self> {
        (fade >= FADE_MIN.0).then(|| Self(fade))
    }

    pub fn raw(self) -> u8 {
        self.0
    }
}

//  [E][E] [E][E]
//  [E][0] [1][2]
//  [E][0] [0][1]
//        /------
//  [E][1]|[0][0]
//  [E][2]|[1][0]
//  [E][1]|[0][0]

pub struct Lightmap {
    width: usize,
    height: usize,
    pub data: Box<[Brightness]>,
}

impl std::ops::Index<usize> for Lightmap {
    type Output = Brightness;
    fn index(&self, i: usize) -> &Self::Output {
        &self.data[i]
    }
}

impl std::ops::IndexMut<usize> for Lightmap {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.data[i]
    }
}

impl Lightmap {
    pub fn new(width: usize, height: usize) -> Self {
        let mut data = vec![LIGHT_MIN; width * height].into_boxed_slice();

        // Create a border around the light map.
        #[rustfmt::skip]
        for x in 0..width {
            data[x                       ] = LIGHT_MAX;
            data[x + (height - 1) * width] = LIGHT_MAX;
        };
        #[rustfmt::skip]
        for y in 0..height {
            data[              y * width] = LIGHT_MAX;
            data[(width - 1) + y * width] = LIGHT_MAX;
        };

        Self {
            width,
            height,
            data,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn fill(
        &mut self,
        fademap: &Fademap,
        orig_probes: impl IntoIterator<Item = impl Into<u16>>,
    ) {
        let stride = self.width;
        assert!(self.data.len() == fademap.data.len());
        assert!(self.data.len() > 4);
        assert!(self.width == fademap.width);
        assert!(stride > 2);

        // Loop through original probes, recording new probes in separate vec.
        let mut probes = Vec::with_capacity(1024);
        for index in orig_probes.into_iter().map(|i| i.into() as usize) {
            assert!(index > stride);
            assert!(index + stride < self.data.len());
            let brightness = self.data[index];
            let fade = fademap.data[index];
            let new_brightness = brightness.apply_fade(fade);

            let offsets = [index - 1, index + 1, index - stride, index + stride];
            for offset in offsets {
                if self.data[offset] < new_brightness {
                    self.data[offset] = new_brightness;
                    probes.push(offset as u16);
                }
            }
        }

        /*let mut i = 0;
        while i < probes.len() {
            unsafe {
                let index = *probes.get_unchecked(i) as usize;
                i += 1;
                //assert!(index > stride);
                //assert!(index + stride < light_map.len());
                let brightness = *self.data.get_unchecked(index);
                let fade = *fademap.data.get_unchecked(index);
                let new_brightness = brightness.apply_fade(fade);

                let offsets = [index - 1, index + 1, index - stride, index + stride];
                for offset in offsets {
                    if *self.data.get_unchecked(offset) < new_brightness {
                        *self.data.get_unchecked_mut(offset) = new_brightness;
                        probes.push(offset as u16);
                    }
                }
            }
        }*/
        // Loop remaining probes, recording new probes in same vec.
        let mut i = 0;
        while i < probes.len() {
            let index = probes[i] as usize;
            i += 1;
            assert!(index > stride);
            assert!(index + stride < self.data.len());
            let brightness = self.data[index];
            let fade = fademap.data[index];
            let new_brightness = brightness.apply_fade(fade);

            let offsets = [index - 1, index + 1, index - stride, index + stride];
            for offset in offsets {
                if self.data[offset] < new_brightness {
                    self.data[offset] = new_brightness;
                    probes.push(offset as u16);
                }
            }
        }
    }
}

pub struct Fademap {
    width: usize,
    height: usize,
    data: Box<[Opacity]>,
}

impl std::ops::Index<usize> for Fademap {
    type Output = Opacity;
    fn index(&self, i: usize) -> &Self::Output {
        &self.data[i]
    }
}

impl std::ops::IndexMut<usize> for Fademap {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.data[i]
    }
}

impl Fademap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![FADE_MIN; width * height].into_boxed_slice(),
        }
    }

    pub fn set(&mut self, ix: usize, iy: usize, fade: Opacity) {
        self.data[ix + iy * self.width] = fade;
    }
}

/*

pub fn create_light_map_base(w: usize, h: usize) -> Box<[u8]> {
    let mut light_map = vec![0; w * h].into_boxed_slice();

    // Create a border around the light map.
    #[rustfmt::skip]
    for x in 0..w {
        light_map[x              ] = LIGHT_MAX;
        light_map[x + (h - 1) * w] = LIGHT_MAX;
    };
    #[rustfmt::skip]
    for y in 0..h {
        light_map[          y * w] = LIGHT_MAX;
        light_map[(w - 1) + y * w] = LIGHT_MAX;
    };

    light_map
}

pub fn create_fade_map_base(w: usize, h: usize) -> Box<[u8]> {
    vec![FADE_MIN; w * h].into_boxed_slice()
}

#[inline(always)]
pub fn fill_light_map(
    stride: usize,
    light_map: &mut Box<[u8]>,
    fade_map: &[u8],
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
*/
