use std::{fs, path::Path};

pub struct Palette(pub Vec<[u8; 3]>);

impl Palette {
    pub fn read(path: &Path) -> Result<Self, std::io::Error> {
        println!("Reading palette {}", path.display());
        let content = fs::read_to_string(path)?;
        let palette = content
            .lines()
            .filter_map(|hex_code| parse_hex_str(hex_code))
            .collect();
        Ok(Self(palette))
    }
}

fn parse_hex_str(s: &str) -> Option<[u8; 3]> {
    let s = s.trim();
    if s.len() != 7 || !s.starts_with('#') {
        return None;
    }
    let r = u8::from_str_radix(&s[1..3], 16).ok()?;
    let g = u8::from_str_radix(&s[3..5], 16).ok()?;
    let b = u8::from_str_radix(&s[5..7], 16).ok()?;
    Some([r, g, b])
}

const LUT_SIZE: usize = 256;

#[derive(Clone)]
pub struct LUT<T>(Box<[T; 3 * LUT_SIZE * LUT_SIZE * LUT_SIZE]>);

impl<T> LUT<T>
where
    T: Copy + Clone,
{
    pub const SIZE: usize = LUT_SIZE;

    pub const fn index(r: usize, g: usize, b: usize) -> usize {
        (r * Self::SIZE * Self::SIZE + g * Self::SIZE + b) * 3
    }

    pub unsafe fn lookup_cell_unchecked(&self, r: usize, g: usize, b: usize) -> [T; 3] {
        let i = Self::index(r, g, b);
        unsafe {
            [
                *self.0.get_unchecked(i),
                *self.0.get_unchecked(i + 1),
                *self.0.get_unchecked(i + 2),
            ]
        }
    }

    pub unsafe fn set_cell_unchecked(
        &mut self,
        r: usize,
        g: usize,
        b: usize,
        new_r: T,
        new_g: T,
        new_b: T,
    ) {
        let index = Self::index(r, g, b);
        unsafe {
            let [r, g, b] = self
                .0
                .get_disjoint_unchecked_mut([index, index + 1, index + 2]);
            *r = new_r;
            *g = new_g;
            *b = new_b;
        }
    }
}

pub fn interpolate(color: [f32; 3], palette: &[[f32; 3]], exponent: f32) -> [f32; 3] {
    match palette.len() {
        0 => return [0.0; 3],
        1 => return palette[0],
        _ => {}
    }

    let mut weighted_sum = [0.0f32; 3];
    let mut weight_total = 0.0f32;

    for &pcolor in palette {
        let dist_sq: f32 = color
            .iter()
            .zip(pcolor)
            .map(|(&a, b)| (a - b).powi(2))
            .sum();

        if dist_sq < f32::EPSILON {
            return pcolor;
        }

        let weight = dist_sq.powf(-exponent / 2.0);
        weight_total += weight;
        for i in 0..3 {
            weighted_sum[i] += weight * pcolor[i];
        }
    }

    let inv = 1.0 / weight_total;
    weighted_sum.map(|v| v * inv)
}

impl LUT<u8> {
    pub fn lookup(&self, r: u8, g: u8, b: u8) -> [u8; 3] {
        let scale = (Self::SIZE - 1) as f32 / 255.0;
        let ri = (r as f32 * scale).round() as usize;
        let gi = (g as f32 * scale).round() as usize;
        let bi = (b as f32 * scale).round() as usize;
        unsafe { self.lookup_cell_unchecked(ri, gi, bi) }
    }

    pub fn from_palette(palette: Palette) -> Self {
        let palette_f32: Vec<[f32; 3]> = palette
            .0
            .iter()
            .map(|&[r, g, b]| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
            .collect();

        let mut lut = Self(
            vec![0u8; 3 * LUT_SIZE * LUT_SIZE * LUT_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
        );
        let scale = 1.0 / (Self::SIZE - 1) as f32;

        for r in 0..Self::SIZE {
            for g in 0..Self::SIZE {
                for b in 0..Self::SIZE {
                    let color = [r as f32 * scale, g as f32 * scale, b as f32 * scale];
                    let result = interpolate(color, &palette_f32, 2.0);
                    unsafe {
                        lut.set_cell_unchecked(
                            r,
                            g,
                            b,
                            (result[0] * 255.0).round() as u8,
                            (result[1] * 255.0).round() as u8,
                            (result[2] * 255.0).round() as u8,
                        );
                    }
                }
            }
        }
        lut
    }
}

impl From<Palette> for LUT<u8> {
    fn from(palette: Palette) -> Self {
        Self::from_palette(palette)
    }
}
