//! The `OS/2` table's parts ttf-parser leaves out: PANOSE, the family class, code pages and the raw style bits.

use crate::core::metadata::fonts::source::{u16_at, u32_at};

#[derive(Clone, Debug, Default)]
pub(super) struct Os2 {
    pub(super) weight: u16,
    pub(super) width: u16,
    pub(super) fs_type: u16,
    /// `sFamilyClass`'s high byte: 1 to 5 and 7 serifs, 8 sans serif, 9 ornamental, 10 scripts, 12 symbolic
    pub(super) class: u8,
    pub(super) panose: [u8; 10],
    pub(super) vendor: String,
    pub(super) fs_selection: u16,
    /// `ulCodePageRange1` and `2`, when the table is version 1 or later
    pub(super) code_pages: Option<u64>,
}

impl Os2 {
    pub(super) fn parse(b: &[u8]) -> Option<Os2> {
        let version = u16_at(b, 0)?;
        let mut panose = [0u8; 10];
        panose.copy_from_slice(b.get(32..42)?);
        let vendor = b.get(58..62)?.iter().filter(|c| c.is_ascii_graphic()).map(|&c| c as char).collect();
        let code_pages = if version >= 1 { Some(u32_at(b, 78)? as u64 | (u32_at(b, 82)? as u64) << 32) } else { None };
        Some(Os2 {
            weight: u16_at(b, 4)?,
            width: u16_at(b, 6)?,
            fs_type: u16_at(b, 8)?,
            class: (u16_at(b, 30)? >> 8) as u8,
            panose,
            vendor,
            fs_selection: u16_at(b, 62)?,
            code_pages,
        })
    }

    pub(super) fn italic(&self) -> bool {
        self.fs_selection & (1 | 1 << 9) != 0
    }

    /// Whether a code page is marked, by its bit in `ulCodePageRange`.
    pub(super) fn code_page(&self, bit: u32) -> bool {
        self.code_pages.is_some_and(|c| c & (1 << bit) != 0)
    }

    /// What the font lets an application do with it once embedded in a document.
    pub(super) fn embedding(&self) -> &'static str {
        let t = self.fs_type;
        if t & 0x000E == 0 {
            "Installable"
        } else if t & 0x0008 != 0 {
            "Editable"
        } else if t & 0x0004 != 0 {
            "Preview and print"
        } else {
            "Restricted"
        }
    }
}

/// `usWeightClass` as CSS takes it; some old fonts count 1 to 9. A value off the hundreds (Roboto Thin and ExtraLight both say 250, so Windows doesn't embolden them) yields to the weight the style names.
pub(super) fn css_weight(w: u16, style: &str) -> u16 {
    let w = match w {
        0 => 400,
        1..=9 => w * 100,
        _ => w.min(1000),
    };
    if w % 100 == 0 {
        return w;
    }
    named_weight(style).unwrap_or(((w + 50) / 100 * 100).clamp(100, 900))
}

/// The weight a style's name says, by the words type designers use.
pub(super) fn named_weight(style: &str) -> Option<u16> {
    let s: String = style.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
    const NAMES: &[(&str, u16)] = &[("hairline", 100), ("thin", 100), ("extralight", 200), ("ultralight", 200), ("semilight", 350), ("light", 300), ("book", 350), ("regular", 400), ("normal", 400), ("medium", 500), ("semibold", 600), ("demibold", 600), ("extrabold", 800), ("ultrabold", 800), ("bold", 700), ("extrablack", 950), ("black", 900), ("heavy", 900)];
    NAMES.iter().find(|(n, _)| s.contains(n)).map(|(_, w)| *w)
}

pub(super) const WIDTHS: [&str; 9] = ["Ultra condensed", "Extra condensed", "Condensed", "Semi condensed", "Normal", "Semi expanded", "Expanded", "Extra expanded", "Ultra expanded"];

pub(super) fn width_name(w: u16) -> &'static str {
    WIDTHS[(w.clamp(1, 9) - 1) as usize]
}
