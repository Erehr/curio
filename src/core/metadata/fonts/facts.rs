//! What a font file says of itself: names, style, weight, category, scripts, axes and licence, for its fields and the font viewer.

use serde_json::{json, Map, Value};
use ttf_parser::{Face, GlyphId, PlatformId, RawFaceTables};

use crate::core::metadata::fonts::container::{Container, Font, Tables, Tag};
use crate::core::metadata::fonts::names::{self, Names};
use crate::core::metadata::fonts::os2::{self, Os2};
use crate::core::metadata::fonts::source::Source;
use crate::core::metadata::fonts::{classify, license, scripts};

const BASE_TABLES: [&Tag; 9] = [b"head", b"hhea", b"maxp", b"OS/2", b"name", b"cmap", b"post", b"fvar", b"hmtx"];
const LAYOUT_TABLES: [&Tag; 2] = [b"GSUB", b"GPOS"];
const OUTLINE_TABLES: [&Tag; 4] = [b"glyf", b"loca", b"CFF ", b"CFF2"];
/// Outlines are read for the shape test only from fonts this small; a font made for Latin is far smaller.
const MAX_OUTLINES: u64 = 4 << 20;
/// A `cmap` listing more is broken (some map every code point to one glyph); the rest is left out.
const MAX_CHARS: usize = 300_000;
const MAX_FACE_NAMES: usize = 64;

#[derive(Clone, Debug)]
pub(super) struct Axis {
    pub(super) tag: String,
    pub(super) name: String,
    pub(super) min: f32,
    pub(super) default: f32,
    pub(super) max: f32,
    pub(super) hidden: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Instance {
    pub(super) name: String,
    pub(super) coords: Vec<f32>,
}

#[derive(Debug)]
pub(crate) struct Facts {
    pub(super) container: Container,
    pub(crate) faces: usize,
    pub(super) names: Names,
    pub(super) os2: Option<Os2>,
    pub(super) weight: u16,
    pub(super) width: u16,
    pub(super) italic: bool,
    pub(super) class: Option<&'static str>,
    pub(super) scripts: Vec<&'static str>,
    pub(super) primary: Option<&'static str>,
    pub(super) axes: Vec<Axis>,
    pub(super) instances: Vec<Instance>,
    pub(super) glyphs: u16,
    pub(super) chars: Vec<u32>,
    pub(super) monospaced: bool,
    pub(super) units_per_em: u16,
    pub(super) features: Vec<String>,
    pub(super) tags: Vec<Tag>,
}

fn raw<'a>(t: &'a Tables, tag: &Tag) -> Option<&'a [u8]> {
    t.get(tag).map(|v| v.as_slice())
}

fn face_of(t: &Tables) -> Result<Face<'_>, String> {
    let (Some(head), Some(hhea), Some(maxp)) = (raw(t, b"head"), raw(t, b"hhea"), raw(t, b"maxp")) else { return Err("The font misses tables every font has".into()) };
    Face::from_raw_tables(RawFaceTables {
        head,
        hhea,
        maxp,
        cmap: raw(t, b"cmap"),
        name: raw(t, b"name"),
        os2: raw(t, b"OS/2"),
        post: raw(t, b"post"),
        fvar: raw(t, b"fvar"),
        hmtx: raw(t, b"hmtx"),
        gsub: raw(t, b"GSUB"),
        gpos: raw(t, b"GPOS"),
        glyf: raw(t, b"glyf"),
        loca: raw(t, b"loca"),
        cff: raw(t, b"CFF "),
        cff2: raw(t, b"CFF2"),
        ..Default::default()
    })
    .map_err(|_| "The font is damaged".to_string())
}

/// Every character the font maps to a glyph, sorted, and whether it maps only through a symbol table.
fn characters(face: &Face) -> (Vec<u32>, bool) {
    let mut chars = Vec::new();
    let mut symbol = false;
    let Some(cmap) = face.tables().cmap else { return (chars, false) };
    for st in cmap.subtables {
        let unicode = st.is_unicode();
        if !unicode && !(st.platform_id == PlatformId::Windows && st.encoding_id == 0) {
            continue;
        }
        symbol |= !unicode;
        st.codepoints(|c| {
            if chars.len() < MAX_CHARS && !(c < 0x20 || (0x7F..0xA0).contains(&c) || c == 0xFFFF) && st.glyph_index(c).is_some() {
                chars.push(c);
            }
        });
    }
    chars.sort_unstable();
    chars.dedup();
    let unicode_letters = chars.iter().any(|&c| c < 0xF000 || c > 0xF0FF);
    (chars, symbol && !unicode_letters)
}

fn axes(face: &Face, names: &Names) -> Vec<Axis> {
    face.variation_axes()
        .into_iter()
        .map(|a| {
            let tag = String::from_utf8_lossy(&a.tag.to_bytes()).trim().to_string();
            let name = match tag.as_str() {
                "wght" => "Weight".to_string(),
                "wdth" => "Width".to_string(),
                "slnt" => "Slant".to_string(),
                "ital" => "Italic".to_string(),
                "opsz" => "Optical size".to_string(),
                _ => names.get(a.name_id).map(String::from).unwrap_or_else(|| tag.clone()),
            };
            Axis { tag, name, min: a.min_value, default: a.def_value, max: a.max_value, hidden: a.hidden }
        })
        .collect()
}

/// `fvar`'s named instances, which ttf-parser leaves out.
fn instances(fvar: Option<&[u8]>, names: &Names) -> Vec<Instance> {
    use crate::core::metadata::fonts::source::{fixed_at, u16_at};
    let Some(b) = fvar else { return vec![] };
    let (Some(axes_at), Some(axis_count), Some(axis_size), Some(count), Some(size)) = (u16_at(b, 4), u16_at(b, 8), u16_at(b, 10), u16_at(b, 12), u16_at(b, 14)) else { return vec![] };
    let start = axes_at as usize + axis_count as usize * axis_size as usize;
    (0..count as usize)
        .filter_map(|i| {
            let r = start + i * size as usize;
            let name = names.get(u16_at(b, r)?)?.to_string();
            let coords = (0..axis_count as usize).map(|a| fixed_at(b, r + 4 + a * 4)).collect::<Option<Vec<f32>>>()?;
            Some(Instance { name, coords })
        })
        .take(256)
        .collect()
}

/// Whether `i`, `m`, `W` and `0` are all there and of one advance.
fn even_advances(face: &Face) -> bool {
    let adv: Vec<Option<u16>> = ['i', 'm', 'W', '0'].iter().map(|c| face.glyph_index(*c).and_then(|g| face.glyph_hor_advance(g))).collect();
    adv.iter().all(|a| a.is_some()) && adv.windows(2).all(|w| w[0] == w[1])
}

/// How much wider the foot of `I` and `l` is than their stem half way up, the narrower of the two: a serif font's both have feet, Tahoma's `I` alone.
fn foot(face: &Face) -> Option<f32> {
    let one = |c: char| -> Option<f32> {
        let g = face.glyph_index(c)?;
        let bbox = face.glyph_bounding_box(g)?;
        let (lo, h) = (bbox.y_min as f32, (bbox.y_max - bbox.y_min) as f32);
        if h <= 0.0 {
            return None;
        }
        let lows = [0.01, 0.02, 0.035, 0.05];
        let mut lines: Vec<f32> = lows.iter().map(|k| lo + k * h).collect();
        lines.push(lo + 0.5 * h);
        let mut w = classify::Widths::new(&lines);
        face.outline_glyph(g, &mut w)?;
        let widths = w.widths();
        let mid = widths[4].filter(|m| *m > 0.0)?;
        let low = widths[..4].iter().flatten().cloned().fold(0.0f32, f32::max);
        Some(low / mid)
    };
    match (one('I'), one('l')) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// What one face of a font file says of itself. `layout` reads its OpenType features too, for the viewer.
pub(crate) fn read<S: Source>(src: S, face_no: usize, layout: bool) -> Result<Facts, String> {
    let mut font = Font::open(src)?;
    let faces = font.faces();
    if face_no >= faces {
        return Err("The file has no such face".into());
    }
    let mut want: Vec<Tag> = BASE_TABLES.iter().map(|t| **t).collect();
    if layout {
        want.extend(LAYOUT_TABLES.iter().map(|t| **t));
    }
    let tables = font.tables(face_no, &want)?;
    let face = face_of(&tables)?;
    let names = Names::parse(raw(&tables, b"name").unwrap_or_default());
    let os2 = raw(&tables, b"OS/2").and_then(Os2::parse);
    let (chars, symbol_cmap) = characters(&face);
    let found = scripts::detect(&scripts::Chars(&chars), os2.as_ref());
    let family = names.family().unwrap_or_default().to_string();
    let primary = scripts::primary(&found, &family, &scripts::Chars(&chars));
    let style = names.style().unwrap_or_default().to_lowercase();
    let italic = os2.as_ref().is_some_and(|o| o.italic()) || face.is_italic() || style.contains("italic") || style.contains("oblique");
    let fixed_pitch = face.tables().post.is_some_and(|p| p.is_monospaced);
    let cjk = found.iter().any(|s| ["Chinese", "Japanese", "Korean"].contains(s));
    let mut clues = classify::Clues {
        family: &family,
        panose: os2.as_ref().map(|o| o.panose).unwrap_or_default(),
        class: os2.as_ref().map(|o| o.class).unwrap_or_default(),
        fixed_pitch,
        even_advances: even_advances(&face),
        symbol_cmap,
        has_letters: !found.is_empty(),
        cjk,
        foot: None,
    };
    let mut class = classify::classify(&clues);
    let small = OUTLINE_TABLES.iter().all(|t| font.table_len(face_no, t).unwrap_or(0) <= MAX_OUTLINES);
    if class.is_none() && found.contains(&"Latin") && small {
        let mut more = tables.clone();
        more.extend(font.tables(face_no, &OUTLINE_TABLES.iter().map(|t| **t).collect::<Vec<_>>())?);
        if let Ok(f) = face_of(&more) {
            clues.foot = foot(&f);
            class = classify::classify(&clues);
        }
    }
    let axes = axes(&face, &names);
    let instances = instances(raw(&tables, b"fvar"), &names);
    let mut features: Vec<String> = Vec::new();
    for t in [face.tables().gsub, face.tables().gpos].into_iter().flatten() {
        for f in t.features {
            let tag = String::from_utf8_lossy(&f.tag.to_bytes()).trim().to_string();
            if !features.contains(&tag) {
                features.push(tag);
            }
        }
    }
    features.sort();
    let weight = os2::css_weight(os2.as_ref().map(|o| o.weight).unwrap_or(400), names.style().unwrap_or_default());
    let width = os2.as_ref().map(|o| o.width).filter(|w| (1..=9).contains(w)).unwrap_or(5);
    let glyphs = face.number_of_glyphs();
    let units_per_em = face.units_per_em();
    Ok(Facts { container: font.container, faces, names, os2, weight, width, italic, class, scripts: found, primary, axes, instances, glyphs, chars, monospaced: fixed_pitch || class == Some(classify::MONO), units_per_em, features, tags: font.tags(face_no) })
}

impl Facts {
    pub(super) fn license(&self) -> Option<String> {
        license::name(self.names.get(names::LICENSE), self.names.get(names::LICENSE_URL))
    }

    pub(super) fn version(&self) -> Option<String> {
        self.names.get(names::VERSION).map(names::short_version).filter(|v| !v.is_empty())
    }

    pub(super) fn outlines(&self) -> &'static str {
        let has = |t: &Tag| self.tags.contains(t);
        if has(b"CFF2") {
            "CFF2"
        } else if has(b"CFF ") {
            "CFF"
        } else if has(b"glyf") {
            "TrueType"
        } else if has(b"sbix") || has(b"CBDT") || has(b"EBDT") {
            "Bitmap"
        } else {
            "None"
        }
    }

    /// The kinds of colour glyphs it has.
    pub(super) fn color(&self) -> Vec<&'static str> {
        [(b"COLR", "COLR"), (b"SVG ", "SVG"), (b"sbix", "sbix"), (b"CBDT", "CBDT")].iter().filter(|(t, _)| self.tags.contains(*t)).map(|(_, n)| *n).collect()
    }

    pub(super) fn format(&self) -> String {
        let cff = self.outlines().starts_with("CFF");
        match self.container {
            Container::Sfnt if cff => "OpenType (CFF)".into(),
            Container::Sfnt => "TrueType".into(),
            Container::Collection if cff => "OpenType collection".into(),
            Container::Collection => "TrueType collection".into(),
            Container::Woff => "WOFF".into(),
            Container::Woff2 => "WOFF2".into(),
            Container::Dfont => "Mac dfont".into(),
        }
    }

    /// The fields Curio keeps, as the extractor answers them.
    pub(crate) fn fields(&self) -> Map<String, Value> {
        let mut m = Map::new();
        let mut text = |k: &str, v: Option<&str>| {
            if let Some(v) = v.map(str::trim).filter(|v| !v.is_empty()) {
                m.insert(k.into(), json!(v));
            }
        };
        text("font_family", self.names.family());
        text("font_style", self.names.style());
        text("designer", self.names.get(names::DESIGNER));
        text("foundry", self.names.get(names::MANUFACTURER));
        text("font_version", self.version().as_deref());
        text("font_license", self.license().as_deref());
        text("font_class", self.class);
        m.insert("font_weight".into(), json!(self.weight));
        m.insert("font_width".into(), json!(os2::width_name(self.width)));
        m.insert("font_italic".into(), json!(self.italic));
        m.insert("glyphs".into(), json!(self.glyphs));
        if !self.scripts.is_empty() {
            m.insert("font_scripts".into(), json!(self.scripts));
        }
        let axes: Vec<&str> = self.axes.iter().filter(|a| !a.hidden).map(|a| a.name.as_str()).collect();
        if !axes.is_empty() {
            m.insert("font_axes".into(), json!(axes));
        }
        if self.faces > 1 {
            m.insert("font_faces".into(), json!(self.faces));
        }
        m
    }

    /// Everything the plugin's pages show: names, style, scripts, axes, features and the characters as ranges.
    pub(crate) fn describe(&self) -> Value {
        let mut names = Map::new();
        for (key, id) in [
            ("family", names::FAMILY),
            ("style", names::SUBFAMILY),
            ("typographicFamily", names::TYPOGRAPHIC_FAMILY),
            ("typographicStyle", names::TYPOGRAPHIC_SUBFAMILY),
            ("full", names::FULL),
            ("postscript", names::POSTSCRIPT),
            ("unique", names::UNIQUE),
            ("version", names::VERSION),
            ("copyright", names::COPYRIGHT),
            ("trademark", names::TRADEMARK),
            ("foundry", names::MANUFACTURER),
            ("designer", names::DESIGNER),
            ("description", names::DESCRIPTION),
            ("vendorUrl", names::VENDOR_URL),
            ("designerUrl", names::DESIGNER_URL),
            ("license", names::LICENSE),
            ("licenseUrl", names::LICENSE_URL),
            ("sample", names::SAMPLE),
        ] {
            if let Some(v) = self.names.get(id) {
                names.insert(key.into(), json!(v));
            }
        }
        let mut ranges: Vec<[u32; 2]> = Vec::new();
        for &c in &self.chars {
            match ranges.last_mut() {
                Some(r) if r[1] + 1 == c => r[1] = c,
                _ => ranges.push([c, c]),
            }
        }
        let container = match self.container {
            Container::Sfnt => "sfnt",
            Container::Collection => "collection",
            Container::Woff => "woff",
            Container::Woff2 => "woff2",
            Container::Dfont => "dfont",
        };
        json!({
            "container": container,
            "format": self.format(),
            "faces": self.faces,
            "family": self.names.family(),
            "style": self.names.style(),
            "names": names,
            "version": self.version(),
            "license": self.license(),
            "weight": self.weight,
            "width": os2::width_name(self.width),
            "italic": self.italic,
            "category": self.class,
            "scripts": self.scripts,
            "primary": self.primary,
            "axes": self.axes.iter().map(|a| json!({ "tag": a.tag, "name": a.name, "min": a.min, "default": a.default, "max": a.max, "hidden": a.hidden })).collect::<Vec<_>>(),
            "instances": self.instances.iter().map(|i| json!({ "name": i.name, "coords": i.coords })).collect::<Vec<_>>(),
            "glyphs": self.glyphs,
            "characters": self.chars.len(),
            "ranges": ranges,
            "features": self.features,
            "outlines": self.outlines(),
            "color": self.color(),
            "monospaced": self.monospaced,
            "unitsPerEm": self.units_per_em,
            "embedding": self.os2.as_ref().map(|o| o.embedding()),
            "vendor": self.os2.as_ref().map(|o| o.vendor.clone()).filter(|v| !v.is_empty()),
        })
    }
}

/// The full name of each face of a collection, for choosing one.
pub(crate) fn face_names<S: Source>(src: S) -> Result<Vec<String>, String> {
    let mut font = Font::open(src)?;
    let mut out = Vec::new();
    for i in 0..font.faces().min(MAX_FACE_NAMES) {
        let t = font.tables(i, &[*b"name"])?;
        let n = Names::parse(raw(&t, b"name").unwrap_or_default());
        out.push(n.get(names::FULL).or(n.family()).map(String::from).unwrap_or_else(|| format!("Face {}", i + 1)));
    }
    Ok(out)
}

/// One face as a font file of its own.
pub(crate) fn face_file<S: Source>(src: S, face: usize) -> Result<Vec<u8>, String> {
    let mut font = Font::open(src)?;
    if face >= font.faces() {
        return Err("The file has no such face".into());
    }
    font.sfnt(face)
}

/// The glyph names of these characters, where the font names its glyphs.
pub(crate) fn glyph_names<S: Source>(src: S, face_no: usize, chars: &[u32]) -> Result<Vec<Option<String>>, String> {
    let mut font = Font::open(src)?;
    let mut want: Vec<Tag> = BASE_TABLES.iter().map(|t| **t).collect();
    if font.table_len(face_no, b"CFF ").is_some_and(|l| l <= MAX_OUTLINES * 2) {
        want.push(*b"CFF ");
    }
    let tables = font.tables(face_no, &want)?;
    let face = face_of(&tables)?;
    Ok(chars.iter().map(|&c| char::from_u32(c).and_then(|ch| face.glyph_index(ch)).and_then(|g: GlyphId| face.glyph_name(g)).map(String::from)).collect())
}
