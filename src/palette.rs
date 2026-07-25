use std::{fs, path::Path};

use crate::interpolation::Interpolation;

pub struct Palette(pub Vec<[u8; 3]>);

impl Palette {
    pub fn read(path: &Path) -> Result<Self, std::io::Error> {
        println!("Reading palette {}", path.display());
        let content = fs::read_to_string(path)?;
        let palette = content
            .lines()
            .filter_map(|hex_code| parse_hex_str(hex_code))
            .collect::<Vec<[u8; 3]>>();

        Ok(Self(palette))
    }
    pub fn to_f32(self) -> Vec<[f32; 3]> {
        self.0
            .iter()
            .map(|&[r, g, b]| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
            .collect()
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

const LUT_SIZE: usize = 64;

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

impl LUT<u8> {
    pub fn lookup(&self, r: u8, g: u8, b: u8) -> [u8; 3] {
        let scale = (Self::SIZE - 1) as f32 / 255.0;
        let ri = (r as f32 * scale).round() as usize;
        let gi = (g as f32 * scale).round() as usize;
        let bi = (b as f32 * scale).round() as usize;
        unsafe { self.lookup_cell_unchecked(ri, gi, bi) }
    }

    pub fn from_interpolation<T: Interpolation + ?Sized>(interpolation: &T) -> Self {
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
                    let result = interpolation.interpolate(color);
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

    pub fn lookup_tril(&self, r: u8, g: u8, b: u8) -> [u8; 3] {
        let scale = (Self::SIZE - 1) as f32 / 255.0;
        let rf = r as f32 * scale;
        let gf = g as f32 * scale;
        let bf = b as f32 * scale;

        let r0 = rf.floor() as usize;
        let g0 = gf.floor() as usize;
        let b0 = bf.floor() as usize;
        let r1 = (r0 + 1).min(Self::SIZE - 1);
        let g1 = (g0 + 1).min(Self::SIZE - 1);
        let b1 = (b0 + 1).min(Self::SIZE - 1);

        let rd = rf - r0 as f32;
        let gd = gf - g0 as f32;
        let bd = bf - b0 as f32;

        unsafe {
            let c000 = self.lookup_cell_unchecked(r0, g0, b0);
            let c100 = self.lookup_cell_unchecked(r1, g0, b0);
            let c010 = self.lookup_cell_unchecked(r0, g1, b0);
            let c110 = self.lookup_cell_unchecked(r1, g1, b0);
            let c001 = self.lookup_cell_unchecked(r0, g0, b1);
            let c101 = self.lookup_cell_unchecked(r1, g0, b1);
            let c011 = self.lookup_cell_unchecked(r0, g1, b1);
            let c111 = self.lookup_cell_unchecked(r1, g1, b1);

            let mut out = [0u8; 3];
            for i in 0..3 {
                let c00 = c000[i] as f32 * (1.0 - rd) + c100[i] as f32 * rd;
                let c10 = c010[i] as f32 * (1.0 - rd) + c110[i] as f32 * rd;
                let c01 = c001[i] as f32 * (1.0 - rd) + c101[i] as f32 * rd;
                let c11 = c011[i] as f32 * (1.0 - rd) + c111[i] as f32 * rd;

                let c0 = c00 * (1.0 - gd) + c10 * gd;
                let c1 = c01 * (1.0 - gd) + c11 * gd;

                out[i] = (c0 * (1.0 - bd) + c1 * bd).round() as u8;
            }
            out
        }
    }
}
