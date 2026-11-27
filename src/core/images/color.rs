//! Colour palette and colour similarity (CIEDE2000).

use crate::core::consts::*;
use crate::core::model::PaletteColor;

// ------------------------------------------------------------------ palette

/// A palette looks at about this many pixels, however large the picture.
const PALETTE_SAMPLE: usize = 40_000;
/// Pixels more transparent than this don't count.
const PALETTE_MIN_ALPHA: u8 = 128;
/// Colours closer than this (CIEDE2000) are one colour.
const PALETTE_MERGE_DE2000: f64 = 2.5;

/// Median-cut quantisation of an RGBA buffer: the most-covering colours first, ratios in percent.
pub fn palette_from_rgba(rgba: &[u8]) -> Vec<PaletteColor> {
    let stride = (rgba.len() / 4 / PALETTE_SAMPLE).max(1);
    let px: Vec<[u8; 3]> = rgba.chunks_exact(4).step_by(stride).filter(|p| p[3] >= PALETTE_MIN_ALPHA).map(|p| [p[0], p[1], p[2]]).collect();
    if px.is_empty() {
        return vec![];
    }
    let total = px.len() as f32;
    let mut boxes = vec![px];
    while boxes.len() < PALETTE_COLORS * 2 {
        // large, varied boxes split first
        let widest = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.len() > 1)
            .map(|(i, b)| {
                let (ch, range) = widest_channel(b);
                (i, ch, range as f64 * (b.len() as f64).sqrt())
            })
            .filter(|(_, _, score)| *score > 0.0)
            .max_by(|x, y| x.2.total_cmp(&y.2));
        let Some((i, ch, _)) = widest else { break };
        let mut b = boxes.swap_remove(i);
        b.sort_unstable_by_key(|p| p[ch]);
        let middle = b[b.len() / 2][ch];
        // identical colours stay together
        let mut cut = b.partition_point(|p| p[ch] < middle);
        if cut == 0 {
            cut = b.partition_point(|p| p[ch] <= middle);
        }
        let right = b.split_off(cut.clamp(1, b.len() - 1));
        boxes.push(b);
        boxes.push(right);
    }
    let mut merged: Vec<(Lab, [u64; 3], usize)> = Vec::new();
    boxes.sort_by_key(|b| std::cmp::Reverse(b.len()));
    for b in &boxes {
        let mut sum = [0u64; 3];
        for p in b {
            for c in 0..3 {
                sum[c] += p[c] as u64;
            }
        }
        let n = b.len() as u64;
        let here = lab([(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]);
        match merged.iter_mut().find(|m| de2000(m.0, here) < PALETTE_MERGE_DE2000) {
            Some(m) => {
                (0..3).for_each(|c| m.1[c] += sum[c]);
                m.2 += b.len();
            }
            None => merged.push((here, sum, b.len())),
        }
    }
    let mut colors: Vec<PaletteColor> = merged
        .iter()
        .map(|(_, sum, count)| {
            let n = *count as u64;
            PaletteColor { color: [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8], ratio: (*count as f32 / total * 1000.0).round() / 10.0 }
        })
        .filter(|c| c.ratio >= PALETTE_MIN_RATIO)
        .collect();
    colors.sort_by(|x, y| y.ratio.total_cmp(&x.ratio));
    colors.truncate(PALETTE_COLORS);
    colors
}

fn widest_channel(b: &[[u8; 3]]) -> (usize, u8) {
    let mut lo = [255u8; 3];
    let mut hi = [0u8; 3];
    for p in b {
        for c in 0..3 {
            lo[c] = lo[c].min(p[c]);
            hi[c] = hi[c].max(p[c]);
        }
    }
    let r = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let ch = if r[0] >= r[1] && r[0] >= r[2] {
        0
    } else if r[1] >= r[2] {
        1
    } else {
        2
    };
    (ch, r[ch])
}

// ------------------------------------------------------------------ Lab and ΔE

#[derive(Clone, Copy, Debug)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

pub fn lab(rgb: [u8; 3]) -> Lab {
    fn lin(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c > 0.04045 {
            ((c + 0.055) / 1.055).powf(2.4)
        } else {
            c / 12.92
        }
    }
    let (r, g, b) = (lin(rgb[0]), lin(rgb[1]), lin(rgb[2]));
    let x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
    let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
    let z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;
    fn f(t: f64) -> f64 {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    }
    let (fx, fy, fz) = (f(x), f(y), f(z));
    Lab { l: 116.0 * fy - 16.0, a: 500.0 * (fx - fy), b: 200.0 * (fy - fz) }
}

pub fn de76(a: Lab, b: Lab) -> f64 {
    ((a.l - b.l).powi(2) + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)).sqrt()
}

/// CIEDE2000.
pub fn de2000(x: Lab, y: Lab) -> f64 {
    use std::f64::consts::PI;
    let rad = |d: f64| d * PI / 180.0;
    let deg = |r: f64| r * 180.0 / PI;
    let c1 = (x.a * x.a + x.b * x.b).sqrt();
    let c2 = (y.a * y.a + y.b * y.b).sqrt();
    let cbar = (c1 + c2) / 2.0;
    let g = 0.5 * (1.0 - (cbar.powi(7) / (cbar.powi(7) + 25f64.powi(7))).sqrt());
    let a1 = (1.0 + g) * x.a;
    let a2 = (1.0 + g) * y.a;
    let c1p = (a1 * a1 + x.b * x.b).sqrt();
    let c2p = (a2 * a2 + y.b * y.b).sqrt();
    let h = |b: f64, a: f64| {
        if b == 0.0 && a == 0.0 {
            0.0
        } else {
            let v = deg(b.atan2(a));
            if v < 0.0 {
                v + 360.0
            } else {
                v
            }
        }
    };
    let h1 = h(x.b, a1);
    let h2 = h(y.b, a2);
    let dl = y.l - x.l;
    let dc = c2p - c1p;
    let dh = if c1p * c2p == 0.0 {
        0.0
    } else if (h2 - h1).abs() <= 180.0 {
        h2 - h1
    } else if h2 - h1 > 180.0 {
        h2 - h1 - 360.0
    } else {
        h2 - h1 + 360.0
    };
    let dhh = 2.0 * (c1p * c2p).sqrt() * (rad(dh) / 2.0).sin();
    let lbar = (x.l + y.l) / 2.0;
    let cbarp = (c1p + c2p) / 2.0;
    let hbar = if c1p * c2p == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * (rad(hbar - 30.0)).cos() + 0.24 * (rad(2.0 * hbar)).cos() + 0.32 * (rad(3.0 * hbar + 6.0)).cos()
        - 0.20 * (rad(4.0 * hbar - 63.0)).cos();
    let dtheta = 30.0 * (-((hbar - 275.0) / 25.0).powi(2)).exp();
    let rc = 2.0 * (cbarp.powi(7) / (cbarp.powi(7) + 25f64.powi(7))).sqrt();
    let sl = 1.0 + (0.015 * (lbar - 50.0).powi(2)) / (20.0 + (lbar - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cbarp;
    let sh = 1.0 + 0.015 * cbarp * t;
    let rt = -(rad(2.0 * dtheta)).sin() * rc;
    ((dl / sl).powi(2) + (dc / sc).powi(2) + (dhh / sh).powi(2) + rt * (dc / sc) * (dhh / sh)).sqrt()
}

/// Does a picture hold this colour? Any colour of its own palette counts exactly; a near colour has
/// to cover a fair part of the picture.
pub fn palette_matches(palette: &[PaletteColor], target: [u8; 3], limit: f64) -> bool {
    let wanted = lab(target);
    palette.iter().any(|p| p.color == target || (p.ratio >= COLOR_SWATCH_MIN_RATIO && de2000(wanted, lab(p.color)) < limit))
}

pub fn palette_grayscale(palette: &[PaletteColor]) -> bool {
    !palette.is_empty()
        && palette.iter().filter(|p| p.ratio >= GRAYSCALE_MIN_RATIO).all(|p| {
            let c = lab(p.color);
            c.a.hypot(c.b) < GRAYSCALE_MAX_CHROMA
        })
}

pub fn palette_to_json(p: &[PaletteColor]) -> String {
    serde_json::to_string(p).unwrap_or_else(|_| "[]".into())
}

pub fn palette_from_json(s: &str) -> Vec<PaletteColor> {
    serde_json::from_str(s).unwrap_or_default()
}

/// The hue of `rgb` on the HSV wheel, in degrees from red; 0 for a grey.
pub fn hue(rgb: [u8; 3]) -> f64 {
    let [r, g, b] = rgb.map(|c| c as f64);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    if max == min {
        return 0.0;
    }
    let d = max - min;
    let h = if max == r { (g - b) / d } else if max == g { 2.0 + (b - r) / d } else { 4.0 + (r - g) / d };
    (h * 60.0).rem_euclid(360.0)
}

/// Where a picture sits in the Spectrum view: its hue (degrees on the HSV wheel), how colourful it is (Lab chroma) and how light (Lab L, 0 to 100).
/// Each palette colour turns the hue by its share times its chroma, so a picture's few strong colours outweigh its greys and near-blacks, and colours of opposite hues cancel out towards grey.
pub fn spectrum_point(palette: &[PaletteColor]) -> Option<(f64, f64, f64)> {
    let total: f64 = palette.iter().map(|p| p.ratio as f64).sum();
    if total <= 0.0 {
        return None;
    }
    let (mut x, mut y, mut l) = (0.0, 0.0, 0.0);
    for p in palette {
        let c = lab(p.color);
        let w = p.ratio as f64 / total;
        let (s, co) = hue(p.color).to_radians().sin_cos();
        let chroma = c.a.hypot(c.b);
        x += w * chroma * co;
        y += w * chroma * s;
        l += w * c.l;
    }
    Some((y.atan2(x).to_degrees().rem_euclid(360.0), x.hypot(y), l))
}
