//! A font's category: what its own data says (PANOSE, then the OS/2 family class), then words in its name, then the shape of its `I`.

pub(super) const SANS: &str = "Sans serif";
pub(super) const SERIF: &str = "Serif";
pub(super) const SLAB: &str = "Slab serif";
pub(super) const MONO: &str = "Monospace";
pub(super) const HAND: &str = "Handwriting";
pub(super) const DISPLAY: &str = "Display";
pub(super) const SYMBOL: &str = "Symbol";

/// The field's options, in the order filters list them.
pub(crate) const ALL: [&str; 7] = [SANS, SERIF, SLAB, MONO, HAND, DISPLAY, SYMBOL];

#[derive(Default)]
pub(super) struct Clues<'a> {
    pub(super) family: &'a str,
    pub(super) panose: [u8; 10],
    /// `sFamilyClass`'s high byte
    pub(super) class: u8,
    pub(super) fixed_pitch: bool,
    /// `i`, `m`, `W` and `0` all present and of one advance
    pub(super) even_advances: bool,
    /// a symbol `cmap` and no letters of any script
    pub(super) symbol_cmap: bool,
    pub(super) has_letters: bool,
    /// a font made for Chinese, Japanese or Korean, whose Latin letters are often all one width
    pub(super) cjk: bool,
    /// how much wider `I` is at its foot than half way up
    pub(super) foot: Option<f32>,
}

const SYMBOL_WORDS: &[&str] = &["dings", "symbol", "symbols", "icons", "icon", "emoji", "webdings", "wingdings", "marlett", "mdl2", "ornaments"];
const MONO_WORDS: &[&str] = &["mono", "monospace", "monospaced", "code", "courier", "typewriter", "terminal", "fixedsys", "console", "consolas", "consola"];
const HAND_WORDS: &[&str] = &["script", "hand", "handwriting", "handwritten", "brush", "marker", "signature", "calligraphy", "calligraphic", "cursive", "chancery", "pen", "ink", "kalam", "pacifico", "caveat"];
const DISPLAY_WORDS: &[&str] = &["stencil", "ornament", "decor", "decorative", "outline", "shadow", "inline", "poster"];
const SANS_WORDS: &[&str] = &["sans", "grotesk", "grotesque", "gothic", "goth", "hei", "heiti", "dotum", "gulim", "arial", "helvetica", "verdana", "tahoma", "segoe", "calibri", "roboto", "inter", "frutiger", "futura", "univers", "ubuntu"];
const SERIF_WORDS: &[&str] = &["serif", "roman", "antiqua", "mincho", "ming", "song", "sun", "batang", "myungjo", "times", "garamond", "baskerville", "bodoni", "caslon", "didot", "georgia", "cambria", "palatino", "minion", "book"];

/// The words of a name, lower case, split at spaces, punctuation and a small letter followed by a capital (`JetBrainsMono`).
fn words(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub(super) fn classify(c: &Clues) -> Option<&'static str> {
    let [family, serif_style, _, proportion, ..] = c.panose;
    // what the font says of itself
    if c.symbol_cmap || family == 5 || c.class == 12 {
        return Some(SYMBOL);
    }
    if c.fixed_pitch || (family == 2 && proportion == 9) || (c.even_advances && !c.cjk) {
        return Some(MONO);
    }
    if family == 3 || c.class == 10 {
        return Some(HAND);
    }
    if family == 4 || c.class == 9 {
        return Some(DISPLAY);
    }
    // words in its name that its data rarely says
    let w = words(c.family);
    let any = |list: &[&str]| w.iter().any(|x| list.contains(&x.as_str()));
    if any(SYMBOL_WORDS) || !c.has_letters {
        return Some(SYMBOL);
    }
    if any(MONO_WORDS) || w.iter().any(|x| x.ends_with("mono") && x != "monotype") {
        return Some(MONO);
    }
    if any(HAND_WORDS) || w.iter().any(|x| x.contains("script") || x.contains("calligraph") || x.starts_with("handwrit")) {
        return Some(HAND);
    }
    if any(DISPLAY_WORDS) {
        return Some(DISPLAY);
    }
    // the shape of its serifs, as PANOSE and the family class describe it
    if family == 2 {
        match serif_style {
            // square serifs without brackets; square cove (Georgia, Palatino) is bracketed
            6 => return Some(SLAB),
            2..=5 | 7..=10 => return Some(SERIF),
            11..=15 => return Some(SANS),
            _ => {}
        }
    }
    match c.class {
        4 | 5 => return Some(SLAB),
        1..=3 | 7 => return Some(SERIF),
        8 => return Some(SANS),
        _ => {}
    }
    if w.iter().any(|x| x == "slab") {
        return Some(SLAB);
    }
    // MS PGothic and MS PMincho run their letters together
    if any(SANS_WORDS) || w.iter().any(|x| x.contains("gothic")) {
        return Some(SANS);
    }
    if any(SERIF_WORDS) || w.iter().any(|x| x.contains("mincho")) {
        return Some(SERIF);
    }
    c.foot.map(|f| if f >= SERIF_FOOT { SERIF } else { SANS })
}

/// A capital I at least this much wider at its foot than half way up has serifs.
pub(super) const SERIF_FOOT: f32 = 1.5;

/// Where a glyph's outline crosses a horizontal line, as the width between its outermost crossings.
#[derive(Default)]
pub(super) struct Widths {
    lines: Vec<f32>,
    hits: Vec<Vec<f32>>,
    at: (f32, f32),
    start: (f32, f32),
}

impl Widths {
    pub(super) fn new(lines: &[f32]) -> Widths {
        Widths { lines: lines.to_vec(), hits: vec![Vec::new(); lines.len()], ..Default::default() }
    }

    fn segment(&mut self, (x0, y0): (f32, f32), (x1, y1): (f32, f32)) {
        for (i, y) in self.lines.iter().enumerate() {
            if (y0 <= *y && y1 > *y) || (y1 <= *y && y0 > *y) {
                self.hits[i].push(x0 + (y - y0) * (x1 - x0) / (y1 - y0));
            }
        }
    }

    pub(super) fn widths(&self) -> Vec<Option<f32>> {
        self.hits.iter().map(|h| (h.len() >= 2).then(|| h.iter().cloned().fold(f32::MIN, f32::max) - h.iter().cloned().fold(f32::MAX, f32::min))).collect()
    }
}

impl ttf_parser::OutlineBuilder for Widths {
    fn move_to(&mut self, x: f32, y: f32) {
        self.at = (x, y);
        self.start = (x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.segment(self.at, (x, y));
        self.at = (x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = self.at;
        let mut prev = p0;
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let u = 1.0 - t;
            let p = (u * u * p0.0 + 2.0 * u * t * x1 + t * t * x, u * u * p0.1 + 2.0 * u * t * y1 + t * t * y);
            self.segment(prev, p);
            prev = p;
        }
        self.at = (x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = self.at;
        let mut prev = p0;
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            let p = (a * p0.0 + b * x1 + c * x2 + d * x, a * p0.1 + b * y1 + c * y2 + d * y);
            self.segment(prev, p);
            prev = p;
        }
        self.at = (x, y);
    }
    fn close(&mut self) {
        let (at, start) = (self.at, self.start);
        self.segment(at, start);
        self.at = start;
    }
}
