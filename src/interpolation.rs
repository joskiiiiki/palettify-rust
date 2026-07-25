use nalgebra::{DMatrix, DVector};
use std::collections::HashSet;

pub trait Interpolation {
    // rgb channels are in range 0 to 1
    fn set_palette(&mut self, palette: Vec<[f32; 3]>);

    fn with(mut self, palette: Vec<[f32; 3]>) -> Self
    where
        Self: Sized,
    {
        self.set_palette(palette);
        self
    }

    fn interpolate(&self, color: [f32; 3]) -> [f32; 3];
}

fn dedup_preserve_order(palette: &mut Vec<[f32; 3]>) {
    let mut seen: HashSet<[u32; 3]> = HashSet::new();
    palette.retain(|&color| {
        // bit-cast floats to bits for Hash/Eq (f32 isn't Hash/Eq due to NaN semantics)
        let key = [color[0].to_bits(), color[1].to_bits(), color[2].to_bits()];
        seen.insert(key) // insert returns true if newly inserted (i.e. first time seen)
    });
}

fn deduped_preserve_order(palette: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let mut seen = HashSet::with_capacity(palette.len());
    palette
        .iter()
        .copied()
        .filter(|color| seen.insert(color.map(f32::to_bits)))
        .collect()
}

fn euclid_dist_sqared(a: [f32; 3], b: [f32; 3]) -> f32 {
    return (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2);
}

pub struct RgbEuclidDistance {
    pub exponent: f32,
    palette: Option<Vec<[f32; 3]>>,
}

impl RgbEuclidDistance {
    pub fn new(exponent: f32) -> Self {
        Self {
            exponent,
            palette: None,
        }
    }
}

impl Interpolation for RgbEuclidDistance {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        let mut palette = palette;
        dedup_preserve_order(&mut palette);
        self.palette = Some(palette)
    }
    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let palette = self.palette.as_ref().expect("palette not set");
        match palette.len() {
            0 => return [0.0; 3],
            1 => return palette[0],
            _ => {}
        }

        let mut weighted_sum = [0.0f32; 3];
        let mut weight_total = 0.0f32;

        for &pcolor in palette {
            let dist_sq: f32 = euclid_dist_sqared(color, pcolor);
            if dist_sq < f32::EPSILON {
                return pcolor;
            }

            let weight = dist_sq.powf(-self.exponent / 2.0);
            weight_total += weight;
            for i in 0..3 {
                weighted_sum[i] += weight * pcolor[i];
            }
        }

        let inv = 1.0 / weight_total;
        weighted_sum.map(|v| v * inv)
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn rgb_to_oklab(c: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = c.map(srgb_to_linear);

    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    [
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    ]
}

fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn oklab_to_rgb(lab: [f32; 3]) -> [f32; 3] {
    let [ll, a, b] = lab;

    // inverse of the OKLab matrix (Lab -> LMS')
    let l_ = ll + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = ll - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = ll - 0.0894841775 * a - 1.2914855480 * b;

    // undo the cube root
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    // inverse of LMS -> linear RGB matrix
    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let bl = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    [linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(bl)]
}

pub struct OkLabEuclidDistance {
    pub exponent: f32,

    palette: Option<Vec<[f32; 3]>>,
}

impl OkLabEuclidDistance {
    pub fn new(exponent: f32) -> Self {
        Self {
            exponent,
            palette: None,
        }
    }
}

impl Interpolation for OkLabEuclidDistance {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        let mut palette = palette;
        dedup_preserve_order(&mut palette);
        let lab_palette = palette.iter().map(|&c| rgb_to_oklab(c)).collect();
        self.palette = Some(lab_palette)
    }
    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let palette = self.palette.as_ref().expect("palette not set");
        match palette.len() {
            0 => return [0.0; 3],
            1 => return oklab_to_rgb(palette[0]),
            _ => {}
        }

        let lab_color = rgb_to_oklab(color);

        let mut weighted_sum = [0.0f32; 3];
        let mut weight_total = 0.0f32;

        for &lab_pcolor in palette {
            let dist_sq: f32 = euclid_dist_sqared(lab_color, lab_pcolor);
            if dist_sq < f32::EPSILON {
                return oklab_to_rgb(lab_pcolor);
            }

            let weight = dist_sq.powf(-self.exponent / 2.0);
            weight_total += weight;
            for i in 0..3 {
                weighted_sum[i] += weight * lab_pcolor[i];
            }
        }

        let inv = 1.0 / weight_total;
        let interpolated_lab = weighted_sum.map(|v| v * inv);
        // println!("lab {interpolated_lab:?}");
        let interpolated_rgb = oklab_to_rgb(interpolated_lab);
        // println!("lab {interpolated_rgb:?}");
        interpolated_rgb
    }
}

pub struct OkLabSoftmin {
    pub temperature: f32, // smaller = sharper/nearest-neighbor-like, larger = smoother/more averaged
    palette: Option<Vec<[f32; 3]>>, // stored as OKLab
}

impl OkLabSoftmin {
    pub fn new(temperature: f32) -> Self {
        Self {
            temperature,
            palette: None,
        }
    }
}

impl Interpolation for OkLabSoftmin {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        let mut palette = palette;
        dedup_preserve_order(&mut palette);
        let lab_palette = palette.iter().map(|&c| rgb_to_oklab(c)).collect();
        self.palette = Some(lab_palette);
    }

    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let palette = self.palette.as_ref().expect("palette not set");

        match palette.len() {
            0 => return [0.0; 3],
            1 => return oklab_to_rgb(palette[0]),
            _ => {}
        }

        let lab_color = rgb_to_oklab(color);
        let mixed_lab = softmin_blend(lab_color, palette, self.temperature);
        oklab_to_rgb(mixed_lab)
    }
}
pub struct Base16RoleAware {
    pub shade_temperature: f32,                      // e.g. 0.15 - smooth
    pub accent_temperature: f32,                     // e.g. 0.03 - sharp
    palette: Option<(Vec<[f32; 3]>, Vec<[f32; 3]>)>, // (shades_lab, accents_lab)
}

impl Base16RoleAware {
    pub fn new(shade_temperature: f32, accent_temperature: f32) -> Self {
        Self {
            shade_temperature,
            accent_temperature,
            palette: None,
        }
    }
}

impl Interpolation for Base16RoleAware {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        assert!(palette.len() == 16, "expected base16 palette");
        let mut shades: Vec<[f32; 3]> = palette[0..8].iter().map(|&c| rgb_to_oklab(c)).collect();
        let mut accents: Vec<[f32; 3]> = palette[8..16].iter().map(|&c| rgb_to_oklab(c)).collect();

        dedup_preserve_order(&mut shades);
        dedup_preserve_order(&mut accents);
        self.palette = Some((shades, accents));
    }

    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let (shades, accents) = self.palette.as_ref().expect("palette not set");
        let lab_color = rgb_to_oklab(color);

        let chroma = chroma_proxy(color); // 0 = gray, higher = saturated

        let shade_lab = softmin_blend(lab_color, shades, self.shade_temperature);
        let accent_lab = softmin_blend(lab_color, accents, self.accent_temperature);

        // mix ratio: more chroma -> lean toward accents
        let t = chroma.clamp(0.0, 1.0);
        let mixed_lab = [
            shade_lab[0] * (1.0 - t) + accent_lab[0] * t,
            shade_lab[1] * (1.0 - t) + accent_lab[1] * t,
            shade_lab[2] * (1.0 - t) + accent_lab[2] * t,
        ];

        oklab_to_rgb(mixed_lab)
    }
}

fn chroma_proxy(rgb: [f32; 3]) -> f32 {
    let max_c = rgb.iter().cloned().fold(f32::MIN, f32::max);
    let min_c = rgb.iter().cloned().fold(f32::MAX, f32::min);
    max_c - min_c
}

fn softmin_blend(lab_color: [f32; 3], group: &[[f32; 3]], temperature: f32) -> [f32; 3] {
    let dists: Vec<f32> = group
        .iter()
        .map(|&p| euclid_dist_sqared(lab_color, p).sqrt())
        .collect();
    let min_dist = dists.iter().cloned().fold(f32::INFINITY, f32::min);

    let mut weighted_sum = [0.0f32; 3];
    let mut weight_total = 0.0f32;
    for (i, &p) in group.iter().enumerate() {
        let w = (-(dists[i] - min_dist) / temperature).exp();
        weight_total += w;
        for c in 0..3 {
            weighted_sum[c] += w * p[c];
        }
    }
    let inv = 1.0 / weight_total;
    weighted_sum.map(|v| v * inv)
}
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub struct Base16Tinted {
    pub shade_temperature: f32,
    pub accent_temperature: f32,
    pub tint_temperature: f32,
    pub tint_gain: f32, // now: chroma value at which tint reaches ~63% of max (tanh scale)
    pub max_tint: f32,
    palette: Option<(Vec<[f32; 3]>, Vec<[f32; 3]>)>,
}

fn nearest(lab_color: [f32; 3], group: &[[f32; 3]]) -> [f32; 3] {
    group
        .iter()
        .cloned()
        .min_by(|a, b| {
            euclid_dist_sqared(lab_color, *a)
                .partial_cmp(&euclid_dist_sqared(lab_color, *b))
                .unwrap()
        })
        .unwrap()
}

impl Base16Tinted {
    pub fn new(
        shade_temperature: f32,
        accent_temperature: f32,
        tint_temperature: f32,
        tint_gain: f32,
        max_tint: f32,
    ) -> Self {
        Self {
            shade_temperature,

            tint_temperature,
            accent_temperature,
            tint_gain,
            max_tint,
            palette: None,
        }
    }
}

impl Interpolation for Base16Tinted {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        assert!(palette.len() == 16, "expected base16 palette (16 entries)");
        let mut shades: Vec<[f32; 3]> = palette[0..8].iter().map(|&c| rgb_to_oklab(c)).collect();
        let mut accents: Vec<[f32; 3]> = palette[8..16].iter().map(|&c| rgb_to_oklab(c)).collect();
        dedup_preserve_order(&mut shades);
        dedup_preserve_order(&mut accents);

        self.palette = Some((shades, accents));
    }

    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let (shades, accents) = self.palette.as_ref().expect("palette not set");
        let lab_color = rgb_to_oklab(color);

        let shade_lab = softmin_blend(lab_color, shades, self.shade_temperature);
        let accent_lab = softmin_blend(lab_color, accents, self.accent_temperature);

        let chroma = (lab_color[1].powi(2) + lab_color[2].powi(2)).sqrt();

        // smooth, monotonic, derivative-continuous saturating curves —
        // no hard clamps, so LUT linear interpolation tracks them cleanly

        // overall shade-vs-accent mix, smoothly ramped instead of hard-clamped chroma
        let t = smoothstep(0.0, 1.0, chroma);

        // tint strength saturates smoothly toward max_tint via tanh
        let tint_strength = self.max_tint * (chroma * self.tint_gain).tanh();

        let mixed_l = shade_lab[0] * (1.0 - t) + accent_lab[0] * t;

        let tint_source = softmin_blend(lab_color, accents, self.tint_temperature); // e.g. 0.01, very sharp        
        let tinted_a = shade_lab[1] * (1.0 - tint_strength) + tint_source[1] * tint_strength;
        let tinted_b = shade_lab[2] * (1.0 - tint_strength) + tint_source[2] * tint_strength;

        let mixed_a = tinted_a * (1.0 - t) + accent_lab[1] * t;
        let mixed_b = tinted_b * (1.0 - t) + accent_lab[2] * t;

        oklab_to_rgb([mixed_l, mixed_a, mixed_b])
    }
}

pub struct RbfInterpolation {
    pub epsilon: f32, // RBF shape parameter: larger = sharper/more local, smaller = smoother/more global
    palette: Option<Vec<[f32; 3]>>,
    weights: Option<Vec<[f32; 3]>>, // one weight vector per palette point, solved once
}

impl RbfInterpolation {
    pub fn new(epsilon: f32) -> Self {
        Self {
            epsilon,
            palette: None,
            weights: None,
        }
    }

    fn kernel(&self, dist_sq: f32) -> f32 {
        // Gaussian RBF
        (-self.epsilon * dist_sq).exp()
    }

    fn solve_weights(&self, palette: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let n = palette.len();

        // build kernel matrix K where K[i][j] = kernel(dist(p_i, p_j))
        let mut k = DMatrix::<f32>::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                let dist_sq = euclid_dist_sqared(palette[i], palette[j]);
                k[(i, j)] = self.kernel(dist_sq);
            }
        }

        let lu = k.lu(); // one decomposition, reused for all 3 channels

        let mut weights = vec![[0.0f32; 3]; n];
        for c in 0..3 {
            let target = DVector::from_iterator(n, palette.iter().map(|p| p[c]));
            let w = lu
                .solve(&target)
                .expect("singular RBF kernel matrix — check for duplicate palette points");
            for i in 0..n {
                weights[i][c] = w[i];
            }
        }
        weights
    }
}

impl Interpolation for RbfInterpolation {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        assert!(!palette.is_empty(), "palette must not be empty");
        let mut palette = palette;
        dedup_preserve_order(&mut palette);
        let weights = self.solve_weights(&palette);
        self.palette = Some(palette);
        self.weights = Some(weights);
    }

    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let palette = self.palette.as_ref().expect("palette not set");
        let weights = self.weights.as_ref().expect("weights not solved");

        if palette.len() == 1 {
            return palette[0];
        }

        let mut result = [0.0f32; 3];
        for (i, &p) in palette.iter().enumerate() {
            let dist_sq = euclid_dist_sqared(color, p);
            let k = self.kernel(dist_sq);
            for c in 0..3 {
                result[c] += weights[i][c] * k;
            }
        }

        // RBF interpolants aren't guaranteed to stay in [0,1] between control points
        // (unlike softmin/IDW, which are convex combinations) — clamp defensively
        result.map(|v| v.clamp(0.0, 1.0))
    }
}
pub struct OkLabRbfInterpolation {
    pub epsilon: f32,
    palette: Option<Vec<[f32; 3]>>, // stored as OKLab
    weights: Option<Vec<[f32; 3]>>, // weights fitted in OKLab space
}

impl OkLabRbfInterpolation {
    pub fn new(epsilon: f32) -> Self {
        Self {
            epsilon,
            palette: None,
            weights: None,
        }
    }

    fn kernel(&self, dist_sq: f32) -> f32 {
        (-self.epsilon * dist_sq).exp()
    }

    fn solve_weights(&self, palette_lab: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let n = palette_lab.len();

        let mut k = DMatrix::<f32>::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                let dist_sq = euclid_dist_sqared(palette_lab[i], palette_lab[j]);
                k[(i, j)] = self.kernel(dist_sq);
            }
        }

        let lu = k.lu();

        let mut weights = vec![[0.0f32; 3]; n];
        for c in 0..3 {
            let target = DVector::from_iterator(n, palette_lab.iter().map(|p| p[c]));
            let w = lu
                .solve(&target)
                .expect("singular RBF kernel matrix — check for duplicate palette points");
            for i in 0..n {
                weights[i][c] = w[i];
            }
        }
        weights
    }
}

impl Interpolation for OkLabRbfInterpolation {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        assert!(!palette.is_empty(), "palette must not be empty");
        let mut palette = palette;
        dedup_preserve_order(&mut palette);
        let palette_lab: Vec<[f32; 3]> = palette.iter().map(|&c| rgb_to_oklab(c)).collect();
        let weights = self.solve_weights(&palette_lab);
        self.palette = Some(palette_lab);
        self.weights = Some(weights);
    }

    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let palette_lab = self.palette.as_ref().expect("palette not set");
        let weights = self.weights.as_ref().expect("weights not solved");

        if palette_lab.len() == 1 {
            return oklab_to_rgb(palette_lab[0]);
        }

        let lab_color = rgb_to_oklab(color);

        let mut result_lab = [0.0f32; 3];
        for (i, &p) in palette_lab.iter().enumerate() {
            let dist_sq = euclid_dist_sqared(lab_color, p);
            let k = self.kernel(dist_sq);
            for c in 0..3 {
                result_lab[c] += weights[i][c] * k;
            }
        }

        oklab_to_rgb(result_lab) // clamping happens naturally inside linear_to_srgb
    }
}

pub struct Base16TintedRbf {
    pub tint_temperature: f32, // sharp softmin temperature for tint hue source (unchanged from Base16Tinted)
    pub tint_gain: f32,
    pub max_tint: f32,
    pub epsilon: f32, // RBF shape parameter, shared by both shade and accent fits
    palette: Option<(RbfGroup, RbfGroup)>, // (shades, accents)
}

struct RbfGroup {
    points: Vec<[f32; 3]>,  // OKLab control points
    weights: Vec<[f32; 3]>, // solved RBF coefficients
}

impl Base16TintedRbf {
    pub fn new(tint_temperature: f32, tint_gain: f32, max_tint: f32, epsilon: f32) -> Self {
        Self {
            tint_temperature,
            tint_gain,
            max_tint,
            epsilon,
            palette: None,
        }
    }

    fn kernel(&self, dist_sq: f32) -> f32 {
        (-self.epsilon * dist_sq).exp()
    }

    fn fit_group(&self, points_lab: Vec<[f32; 3]>) -> RbfGroup {
        let n = points_lab.len();

        let mut k = DMatrix::<f32>::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                let dist_sq = euclid_dist_sqared(points_lab[i], points_lab[j]);
                k[(i, j)] = self.kernel(dist_sq);
            }
        }

        let lu = k.lu();
        let mut weights = vec![[0.0f32; 3]; n];
        for c in 0..3 {
            let target = DVector::from_iterator(n, points_lab.iter().map(|p| p[c]));
            let w = lu
                .solve(&target)
                .expect("singular RBF kernel matrix — check for duplicate palette points");
            for i in 0..n {
                weights[i][c] = w[i];
            }
        }

        RbfGroup {
            points: points_lab,
            weights,
        }
    }

    fn eval_group(&self, group: &RbfGroup, lab_color: [f32; 3]) -> [f32; 3] {
        let mut result = [0.0f32; 3];
        for (i, &p) in group.points.iter().enumerate() {
            let dist_sq = euclid_dist_sqared(lab_color, p);
            let k = self.kernel(dist_sq);
            for c in 0..3 {
                result[c] += group.weights[i][c] * k;
            }
        }
        result
    }
}

impl Interpolation for Base16TintedRbf {
    fn set_palette(&mut self, palette: Vec<[f32; 3]>) {
        assert!(palette.len() == 16, "expected base16 palette (16 entries)");

        let mut shades_lab: Vec<[f32; 3]> =
            palette[0..8].iter().map(|&c| rgb_to_oklab(c)).collect();
        let mut accents_lab: Vec<[f32; 3]> =
            palette[8..16].iter().map(|&c| rgb_to_oklab(c)).collect();

        dedup_preserve_order(&mut shades_lab);
        dedup_preserve_order(&mut accents_lab);
        let shade_group = self.fit_group(shades_lab);
        let accent_group = self.fit_group(accents_lab);

        self.palette = Some((shade_group, accent_group));
    }

    fn interpolate(&self, color: [f32; 3]) -> [f32; 3] {
        let (shades, accents) = self.palette.as_ref().expect("palette not set");
        let lab_color = rgb_to_oklab(color);

        let shade_lab = self.eval_group(shades, lab_color);
        let accent_lab = self.eval_group(accents, lab_color);

        let chroma = (lab_color[1].powi(2) + lab_color[2].powi(2)).sqrt();

        let t = smoothstep(0.0, 1.0, chroma);
        let tint_strength = self.max_tint * (chroma * self.tint_gain).tanh();

        let mixed_l = shade_lab[0] * (1.0 - t) + accent_lab[0] * t;

        // tint source stays as sharp softmin over the raw accent palette points,
        // same mechanism as Base16Tinted — RBF's accent_group is a smooth fit,
        // not a discrete set to argmin over, so we keep the original accent points here
        let tint_source = softmin_blend(lab_color, &accents.points, self.tint_temperature);

        let tinted_a = shade_lab[1] * (1.0 - tint_strength) + tint_source[1] * tint_strength;
        let tinted_b = shade_lab[2] * (1.0 - tint_strength) + tint_source[2] * tint_strength;

        let mixed_a = tinted_a * (1.0 - t) + accent_lab[1] * t;
        let mixed_b = tinted_b * (1.0 - t) + accent_lab[2] * t;

        oklab_to_rgb([mixed_l, mixed_a, mixed_b])
    }
}
