//! Which writing systems a font has the letters for, from its characters rather than its own claims (`ulUnicodeRange` is often wrong).

use crate::core::metadata::fonts::os2::Os2;

/// The characters a font maps, sorted.
pub(super) struct Chars<'a>(pub &'a [u32]);

impl Chars<'_> {
    pub(super) fn count(&self, lo: u32, hi: u32) -> usize {
        self.0.partition_point(|&c| c <= hi) - self.0.partition_point(|&c| c < lo)
    }

    /// The share of these ranges the font has.
    fn share(&self, ranges: &[(u32, u32)]) -> f32 {
        let (have, all) = ranges.iter().fold((0, 0), |(h, a), &(lo, hi)| (h + self.count(lo, hi), a + (hi - lo + 1) as usize));
        have as f32 / all as f32
    }
}

/// A script's basic letters, and the share of them a font needs: lower where the range has unassigned code points.
const ALPHABETS: &[(&str, &[(u32, u32)], f32)] = &[
    ("Latin", &[(0x41, 0x5A), (0x61, 0x7A)], 0.9),
    ("Greek", &[(0x391, 0x3A1), (0x3A3, 0x3A9), (0x3B1, 0x3C9)], 0.9),
    ("Cyrillic", &[(0x410, 0x44F)], 0.9),
    ("Armenian", &[(0x531, 0x556), (0x561, 0x586)], 0.9),
    ("Georgian", &[(0x10D0, 0x10F0)], 0.9),
    ("Hebrew", &[(0x5D0, 0x5EA)], 0.9),
    ("Arabic", &[(0x621, 0x63A), (0x641, 0x64A)], 0.9),
    ("Thai", &[(0xE01, 0xE2E)], 0.9),
    ("Lao", &[(0xE81, 0xEAE)], 0.5),
    ("Khmer", &[(0x1780, 0x17A2)], 0.9),
    ("Myanmar", &[(0x1000, 0x1021)], 0.9),
    ("Tibetan", &[(0xF40, 0xF69)], 0.8),
    ("Ethiopic", &[(0x1200, 0x1248)], 0.8),
    ("Devanagari", &[(0x905, 0x939)], 0.8),
    ("Bengali", &[(0x985, 0x9B9)], 0.6),
    ("Gurmukhi", &[(0xA05, 0xA39)], 0.5),
    ("Gujarati", &[(0xA85, 0xAB9)], 0.6),
    ("Tamil", &[(0xB85, 0xBB9)], 0.5),
    ("Telugu", &[(0xC05, 0xC39)], 0.6),
    ("Kannada", &[(0xC85, 0xCB9)], 0.6),
    ("Malayalam", &[(0xD05, 0xD39)], 0.6),
    ("Sinhala", &[(0xD85, 0xDC6)], 0.6),
];

/// Every script this reader names, in the order it lists them: the field's options.
pub(crate) fn all() -> Vec<&'static str> {
    let mut v: Vec<&str> = ALPHABETS.iter().map(|(n, _, _)| *n).collect();
    v.extend(["Chinese", "Japanese", "Korean"]);
    v
}

const KANA: &[(u32, u32)] = &[(0x3041, 0x3093), (0x30A1, 0x30F3)];
const MIN_IDEOGRAPHS: usize = 2000;
const MIN_HANGUL: usize = 2000;
// ulCodePageRange1 bits
const CP_JAPANESE: u32 = 17;
const CP_SIMPLIFIED: u32 = 18;
const CP_KOREAN: u32 = 19;
const CP_TRADITIONAL: u32 = 20;
const CP_JOHAB: u32 = 21;

pub(super) fn detect(chars: &Chars, os2: Option<&Os2>) -> Vec<&'static str> {
    let mut out: Vec<&str> = ALPHABETS.iter().filter(|(_, r, min)| chars.share(r) >= *min).map(|(n, _, _)| *n).collect();
    // the main block, extension A and the supplementary plane's extensions, which fonts such as SimSun-ExtB hold alone
    let han = chars.count(0x4E00, 0x9FFF) + chars.count(0x3400, 0x4DBF) + chars.count(0x20000, 0x3134F);
    let hangul = chars.count(0xAC00, 0xD7A3);
    let kana = chars.share(KANA) >= 0.9;
    let cjk_marked = os2.is_some_and(|o| (CP_JAPANESE..=CP_JOHAB).any(|b| o.code_page(b)));
    let (chinese, japanese, korean) = if let (true, Some(o)) = (cjk_marked, os2) {
        // ideographs alone don't say whose they are; the code pages the font was made for do
        ((o.code_page(CP_SIMPLIFIED) || o.code_page(CP_TRADITIONAL)) && han >= MIN_IDEOGRAPHS, o.code_page(CP_JAPANESE) && kana, (o.code_page(CP_KOREAN) || o.code_page(CP_JOHAB)) && hangul >= MIN_HANGUL)
    } else {
        let japanese = kana;
        let korean = hangul >= MIN_HANGUL;
        (han >= MIN_IDEOGRAPHS && !japanese && !korean, japanese, korean)
    };
    for (on, name) in [(chinese, "Chinese"), (japanese, "Japanese"), (korean, "Korean")] {
        if on {
            out.push(name);
        }
    }
    out
}

/// The script the font is made for, which its samples are written in: one its family names, then Chinese, Japanese or Korean (a font with thousands of ideographs is made for them), then Latin, then the one with most letters.
pub(super) fn primary(scripts: &[&'static str], family: &str, chars: &Chars) -> Option<&'static str> {
    let words: Vec<String> = family.split(|c: char| !c.is_alphanumeric()).map(|w| w.to_lowercase()).filter(|w| !w.is_empty()).collect();
    let named = |s: &str| -> bool {
        let aliases: &[&str] = match s {
            "Chinese" => &["chinese", "sc", "tc", "hk", "cn", "tw"],
            "Japanese" => &["japanese", "jp"],
            "Korean" => &["korean", "kr"],
            _ => &[],
        };
        words.iter().any(|w| *w == s.to_lowercase() || aliases.contains(&w.as_str()))
    };
    if let Some(s) = scripts.iter().find(|s| **s != "Latin" && named(s)) {
        return Some(s);
    }
    let cjk: Vec<&'static str> = scripts.iter().filter(|s| ["Chinese", "Japanese", "Korean"].contains(s)).copied().collect();
    if !cjk.is_empty() {
        // a pan-CJK font naming none of them: Chinese, the largest
        return Some(if cjk.contains(&"Chinese") { "Chinese" } else { cjk[0] });
    }
    if scripts.contains(&"Latin") {
        return Some("Latin");
    }
    scripts.iter().max_by_key(|s| ALPHABETS.iter().find(|(n, _, _)| n == *s).map(|(_, r, _)| r.iter().map(|&(lo, hi)| chars.count(lo, hi)).sum::<usize>()).unwrap_or(0)).copied()
}
