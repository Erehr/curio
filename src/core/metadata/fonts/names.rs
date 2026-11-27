//! The `name` table: each name in English where the font has it, else in the language it has.

use std::collections::BTreeMap;

use crate::core::metadata::fonts::source::u16_at;

pub(super) const COPYRIGHT: u16 = 0;
pub(super) const FAMILY: u16 = 1;
pub(super) const SUBFAMILY: u16 = 2;
pub(super) const UNIQUE: u16 = 3;
pub(super) const FULL: u16 = 4;
pub(super) const VERSION: u16 = 5;
pub(super) const POSTSCRIPT: u16 = 6;
pub(super) const TRADEMARK: u16 = 7;
pub(super) const MANUFACTURER: u16 = 8;
pub(super) const DESIGNER: u16 = 9;
pub(super) const DESCRIPTION: u16 = 10;
pub(super) const VENDOR_URL: u16 = 11;
pub(super) const DESIGNER_URL: u16 = 12;
pub(super) const LICENSE: u16 = 13;
pub(super) const LICENSE_URL: u16 = 14;
pub(super) const TYPOGRAPHIC_FAMILY: u16 = 16;
pub(super) const TYPOGRAPHIC_SUBFAMILY: u16 = 17;
pub(super) const SAMPLE: u16 = 19;

#[derive(Clone, Debug, Default)]
pub(super) struct Names(BTreeMap<u16, (u8, String)>);

/// Mac OS Roman's upper half, which old Mac names are written in.
const MAC_ROMAN: &str = "ÄÅÇÉÑÖÜáàâäãåçéèêëíìîïñóòôöõúùûü†°¢£§•¶ß®©™´¨≠ÆØ∞±≤≥¥µ∂∑∏π∫ªºΩæø¿¡¬√ƒ≈∆«»…\u{A0}ÀÃÕŒœ–—“”‘’÷◊ÿŸ⁄€‹›ﬁﬂ‡·‚„‰ÂÊÁËÈÍÎÏÌÓÔ\u{F8FF}ÒÚÛÙıˆ˜¯˘˙˚¸˝˛ˇ";

impl Names {
    pub(super) fn parse(data: &[u8]) -> Names {
        let mut out = BTreeMap::new();
        let (Some(count), Some(strings)) = (u16_at(data, 2), u16_at(data, 4)) else { return Names(out) };
        for i in 0..count as usize {
            let r = 6 + i * 12;
            let (Some(platform), Some(encoding), Some(language), Some(id), Some(len), Some(at)) = (u16_at(data, r), u16_at(data, r + 2), u16_at(data, r + 4), u16_at(data, r + 6), u16_at(data, r + 8), u16_at(data, r + 10)) else { break };
            let score = match (platform, encoding, language) {
                (3, 1 | 10, 0x0409) => 100,
                (3, 1 | 10, l) if l & 0x3FF == 0x09 => 90,
                (0, _, _) => 80,
                (1, 0, 0) => 70,
                (3, 1 | 10, _) => 60,
                (3, 0, _) => 50,
                _ => continue,
            };
            if out.get(&id).is_some_and(|(s, _): &(u8, String)| *s >= score) {
                continue;
            }
            let start = strings as usize + at as usize;
            let Some(bytes) = data.get(start..start + len as usize) else { continue };
            let text = if platform == 1 { mac_roman(bytes) } else { utf16(bytes) };
            let text = text.replace('\0', "").trim().to_string();
            if !text.is_empty() {
                out.insert(id, (score, text));
            }
        }
        Names(out)
    }

    pub(super) fn get(&self, id: u16) -> Option<&str> {
        self.0.get(&id).map(|(_, s)| s.as_str())
    }

    /// The family as the font groups its styles: the typographic one, else the plain one.
    pub(super) fn family(&self) -> Option<&str> {
        self.get(TYPOGRAPHIC_FAMILY).or(self.get(FAMILY))
    }

    pub(super) fn style(&self) -> Option<&str> {
        self.get(TYPOGRAPHIC_SUBFAMILY).or(self.get(SUBFAMILY))
    }

}

fn utf16(b: &[u8]) -> String {
    let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&units)
}

fn mac_roman(b: &[u8]) -> String {
    let high: Vec<char> = MAC_ROMAN.chars().collect();
    b.iter().map(|&c| if c < 0x80 { c as char } else { high.get(c as usize - 0x80).copied().unwrap_or('\u{FFFD}') }).collect()
}

/// A version as people say it: "Version 2.137; ttfautohint (v1.8.4)" is 2.137.
pub(super) fn short_version(v: &str) -> String {
    let v = v.split(';').next().unwrap_or(v).trim();
    let lower = v.to_lowercase();
    let v = if lower.starts_with("version") { v["version".len()..].trim() } else { v };
    v.trim_start_matches(['v', 'V']).trim().to_string()
}
