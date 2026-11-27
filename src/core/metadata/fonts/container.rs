//! The files fonts come in: a bare TrueType or OpenType font, a collection (TTC, OTC), WOFF, WOFF2 and a Mac dfont, each read as faces made of tables.

use std::collections::BTreeMap;
use std::io::Read;

use crate::core::metadata::fonts::source::*;

pub(super) type Tag = [u8; 4];
pub(super) type Tables = BTreeMap<Tag, Vec<u8>>;

pub(super) const NOT_A_FONT: &str = "Not a font file Curio reads";
const MAX_TABLES: usize = 512;
const MAX_FACES: usize = 256;
/// A table larger than this is left out; none this reader needs comes near it.
pub(super) const MAX_TABLE: u64 = 64 << 20;
const MAX_WOFF2_STREAM: u64 = 160 << 20;
/// The most a face rebuilt as a font file of its own may unpack to, all its tables together.
const MAX_FACE: u64 = 256 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Container {
    Sfnt,
    Collection,
    Woff,
    Woff2,
    Dfont,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum How {
    Plain,
    Zlib,
    /// in WOFF2's one decompressed stream; a transformed `glyf`, `loca` or `hmtx` isn't rebuilt
    Woff2 { transformed: bool },
}

#[derive(Clone, Debug)]
struct Entry {
    tag: Tag,
    offset: u64,
    length: u64,
    stored: u64,
    how: How,
}

#[derive(Clone, Debug)]
struct Face {
    flavor: u32,
    entries: Vec<Entry>,
}

pub(super) struct Font<S: Source> {
    src: S,
    pub(super) container: Container,
    faces: Vec<Face>,
    /// where WOFF2's compressed stream is and how long, and the stream once read
    woff2: Option<(u64, u64)>,
    stream: Option<Vec<u8>>,
}

/// WOFF2's table tags by their index in a directory entry's flags.
pub(crate) const WOFF2_TAGS: [&[u8; 4]; 63] = [
    b"cmap", b"head", b"hhea", b"hmtx", b"maxp", b"name", b"OS/2", b"post", b"cvt ", b"fpgm", b"glyf", b"loca", b"prep", b"CFF ", b"VORG", b"EBDT", b"EBLC", b"gasp", b"hdmx", b"kern", b"LTSH", b"PCLT", b"VDMX", b"vhea", b"vmtx", b"BASE", b"GDEF", b"GPOS", b"GSUB", b"EBSC", b"JSTF", b"MATH", b"CBDT", b"CBLC", b"COLR", b"CPAL", b"SVG ", b"sbix", b"acnt", b"avar", b"bdat", b"bloc", b"bsln", b"cvar", b"fdsc", b"feat", b"fmtx", b"fvar", b"gvar", b"hsty", b"just", b"lcar", b"mort", b"morx", b"opbd", b"prop", b"trak", b"Zapf", b"Silf", b"Glat", b"Gloc", b"Feat", b"Sill",
];

impl<S: Source> Font<S> {
    pub(super) fn open(mut src: S) -> Result<Self, String> {
        let size = src.size();
        if size < 12 {
            return Err(NOT_A_FONT.into());
        }
        let head = src.read_at(0, 48.min(size) as usize)?;
        let mut f = Font { src, container: Container::Sfnt, faces: vec![], woff2: None, stream: None };
        match &head[0..4] {
            b"ttcf" => {
                f.container = Container::Collection;
                f.collection()?;
            }
            b"wOFF" => {
                f.container = Container::Woff;
                f.woff(&head)?;
            }
            b"wOF2" => {
                f.container = Container::Woff2;
                f.woff2_directory(&head)?;
            }
            [0, 1, 0, 0] | b"true" | b"OTTO" | b"typ1" => {
                let face = f.directory(0, 0)?;
                f.faces.push(face);
            }
            _ => {
                f.container = Container::Dfont;
                f.dfont().map_err(|_| NOT_A_FONT.to_string())?;
            }
        }
        if f.faces.is_empty() {
            return Err(NOT_A_FONT.into());
        }
        Ok(f)
    }

    pub(super) fn faces(&self) -> usize {
        self.faces.len()
    }

    /// The face's outline kind as its file states it: `true` or 0x00010000 for TrueType, `OTTO` for CFF.
    pub(super) fn flavor(&self, face: usize) -> u32 {
        self.faces.get(face).map(|f| f.flavor).unwrap_or(0)
    }

    pub(super) fn table_len(&self, face: usize, tag: &Tag) -> Option<u64> {
        self.faces.get(face)?.entries.iter().find(|e| &e.tag == tag).map(|e| e.length)
    }

    pub(super) fn tags(&self, face: usize) -> Vec<Tag> {
        self.faces.get(face).map(|f| f.entries.iter().map(|e| e.tag).collect()).unwrap_or_default()
    }

    /// The tables of `want` the face has, as their bytes; one that can't be read is left out.
    pub(super) fn tables(&mut self, face: usize, want: &[Tag]) -> Result<Tables, String> {
        let entries = self.faces.get(face).ok_or("The file has no such face")?.entries.clone();
        let mut out = Tables::new();
        for e in entries.iter().filter(|e| want.contains(&e.tag)) {
            if let Some(b) = self.load(e)? {
                out.insert(e.tag, b);
            }
        }
        Ok(out)
    }

    /// One face as a font file of its own, for a page to load: every table it has, a transformed one aside.
    pub(super) fn sfnt(&mut self, face: usize) -> Result<Vec<u8>, String> {
        let tags = self.tags(face);
        let total: u64 = self.faces.get(face).map_or(0, |f| f.entries.iter().map(|e| e.length).sum());
        if total > MAX_FACE {
            return Err("The font is too large to read".into());
        }
        let tables = self.tables(face, &tags)?;
        Ok(write_sfnt(self.flavor(face), &tables))
    }

    fn load(&mut self, e: &Entry) -> Result<Option<Vec<u8>>, String> {
        if e.length > MAX_TABLE || e.stored > MAX_TABLE {
            return Ok(None);
        }
        match e.how {
            How::Plain => {
                let size = self.src.size();
                // some fonts count the last table's padding past the end of the file; more than that is a file cut short
                if e.offset > size || e.offset + e.length > size + 3 {
                    return Err(ENDS_EARLY.into());
                }
                let len = e.length.min(size - e.offset) as usize;
                self.src.read_at(e.offset, len).map(Some)
            }
            How::Zlib => {
                let raw = self.src.read_at(e.offset, e.stored as usize)?;
                Ok(miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&raw, e.length as usize).ok().filter(|b| b.len() as u64 == e.length))
            }
            How::Woff2 { transformed: true } => Ok(None),
            How::Woff2 { transformed: false } => {
                let stream = self.stream()?;
                let (start, end) = (e.offset as usize, (e.offset + e.length) as usize);
                Ok(stream.get(start..end).map(|b| b.to_vec()))
            }
        }
    }

    fn stream(&mut self) -> Result<&Vec<u8>, String> {
        if self.stream.is_none() {
            let (at, len) = self.woff2.ok_or(NOT_A_FONT)?;
            if len > MAX_WOFF2_STREAM {
                return Err("The font is too large to read".into());
            }
            let packed = self.src.read_at(at, len as usize)?;
            let mut out = Vec::new();
            brotli_decompressor::Decompressor::new(&packed[..], 4096).take(MAX_WOFF2_STREAM).read_to_end(&mut out).map_err(|_| "The WOFF2 font's data is damaged".to_string())?;
            self.stream = Some(out);
        }
        Ok(self.stream.as_ref().unwrap())
    }

    /// An sfnt table directory at `at`, whose offsets count from `base`.
    fn directory(&mut self, at: u64, base: u64) -> Result<Face, String> {
        let head = self.src.read_at(at, 12)?;
        let flavor = u32_at(&head, 0).unwrap();
        let n = u16_at(&head, 4).unwrap() as usize;
        if n == 0 || n > MAX_TABLES {
            return Err(NOT_A_FONT.into());
        }
        let dir = self.src.read_at(at + 12, n * 16)?;
        let entries = (0..n)
            .map(|i| {
                let r = i * 16;
                let length = u32_at(&dir, r + 12).unwrap() as u64;
                Entry { tag: tag_at(&dir, r).unwrap(), offset: base + u32_at(&dir, r + 8).unwrap() as u64, length, stored: length, how: How::Plain }
            })
            .collect();
        Ok(Face { flavor, entries })
    }

    fn collection(&mut self) -> Result<(), String> {
        let head = self.src.read_at(0, 12)?;
        let n = u32_at(&head, 8).unwrap() as usize;
        if n == 0 || n > MAX_FACES {
            return Err(NOT_A_FONT.into());
        }
        let offsets = self.src.read_at(12, n * 4)?;
        for i in 0..n {
            let face = self.directory(u32_at(&offsets, i * 4).unwrap() as u64, 0)?;
            self.faces.push(face);
        }
        Ok(())
    }

    fn woff(&mut self, head: &[u8]) -> Result<(), String> {
        let flavor = u32_at(head, 4).ok_or(NOT_A_FONT)?;
        let n = u16_at(head, 12).ok_or(NOT_A_FONT)? as usize;
        if n == 0 || n > MAX_TABLES {
            return Err(NOT_A_FONT.into());
        }
        let dir = self.src.read_at(44, n * 20)?;
        let entries = (0..n)
            .map(|i| {
                let r = i * 20;
                let (stored, length) = (u32_at(&dir, r + 8).unwrap() as u64, u32_at(&dir, r + 12).unwrap() as u64);
                Entry { tag: tag_at(&dir, r).unwrap(), offset: u32_at(&dir, r + 4).unwrap() as u64, length, stored, how: if stored < length { How::Zlib } else { How::Plain } }
            })
            .collect();
        self.faces.push(Face { flavor, entries });
        Ok(())
    }

    fn woff2_directory(&mut self, head: &[u8]) -> Result<(), String> {
        if head.len() < 48 {
            return Err(NOT_A_FONT.into());
        }
        let flavor = u32_at(head, 4).unwrap();
        let n = u16_at(head, 12).unwrap() as usize;
        let packed = u32_at(head, 20).unwrap() as u64;
        if n == 0 || n > MAX_TABLES {
            return Err(NOT_A_FONT.into());
        }
        let size = self.src.size();
        let dir = self.src.read_at(48, (size - 48).min(1 << 20) as usize)?;
        let mut c = Cursor { b: &dir, at: 0 };
        let mut all = Vec::with_capacity(n);
        let mut offset = 0u64;
        for _ in 0..n {
            let flags = c.u8()?;
            let tag = match flags & 63 {
                63 => c.tag()?,
                i => *WOFF2_TAGS[i as usize],
            };
            let version = flags >> 6;
            let original = c.base128()? as u64;
            let transformed = if &tag == b"glyf" || &tag == b"loca" { version != 3 } else { version != 0 };
            let length = if transformed { c.base128()? as u64 } else { original };
            all.push(Entry { tag, offset, length, stored: length, how: How::Woff2 { transformed } });
            offset += length;
        }
        if flavor == u32::from_be_bytes(*b"ttcf") {
            c.u32()?;
            let faces = c.u255()? as usize;
            if faces == 0 || faces > MAX_FACES {
                return Err(NOT_A_FONT.into());
            }
            for _ in 0..faces {
                let count = c.u255()? as usize;
                let flavor = c.u32()?;
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    entries.push(all.get(c.u255()? as usize).ok_or(NOT_A_FONT)?.clone());
                }
                self.faces.push(Face { flavor, entries });
            }
            self.container = Container::Woff2;
        } else {
            self.faces.push(Face { flavor, entries: all });
        }
        self.woff2 = Some((48 + c.at as u64, packed));
        Ok(())
    }

    /// A Mac resource file in the data fork: each `sfnt` resource is a face.
    fn dfont(&mut self) -> Result<(), String> {
        let size = self.src.size();
        let head = self.src.read_at(0, 16)?;
        let (data, map, map_len) = (u32_at(&head, 0).unwrap() as u64, u32_at(&head, 4).unwrap() as u64, u32_at(&head, 12).unwrap() as u64);
        if data < 16 || map + map_len > size || map_len < 30 || map_len > 16 << 20 {
            return Err(NOT_A_FONT.into());
        }
        let m = self.src.read_at(map, map_len as usize)?;
        let list = u16_at(&m, 24).ok_or(NOT_A_FONT)? as usize;
        let types = u16_at(&m, list).ok_or(NOT_A_FONT)? as usize + 1;
        let mut starts = Vec::new();
        for t in 0..types {
            let e = list + 2 + t * 8;
            if m.get(e..e + 4) != Some(b"sfnt") {
                continue;
            }
            let count = u16_at(&m, e + 4).ok_or(NOT_A_FONT)? as usize + 1;
            let refs = list + u16_at(&m, e + 6).ok_or(NOT_A_FONT)? as usize;
            for r in 0..count.min(MAX_FACES) {
                let at = u24_at(&m, refs + r * 12 + 5).ok_or(NOT_A_FONT)? as u64;
                starts.push(data + at + 4);
            }
        }
        for s in starts {
            let face = self.directory(s, s)?;
            self.faces.push(face);
        }
        Ok(())
    }
}

struct Cursor<'a> {
    b: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let v = *self.b.get(self.at).ok_or(ENDS_EARLY)?;
        self.at += 1;
        Ok(v)
    }

    fn u16(&mut self) -> Result<u16, String> {
        let v = u16_at(self.b, self.at).ok_or(ENDS_EARLY)?;
        self.at += 2;
        Ok(v)
    }

    fn u32(&mut self) -> Result<u32, String> {
        let v = u32_at(self.b, self.at).ok_or(ENDS_EARLY)?;
        self.at += 4;
        Ok(v)
    }

    fn tag(&mut self) -> Result<Tag, String> {
        Ok(self.u32()?.to_be_bytes())
    }

    /// WOFF2's UIntBase128: seven bits a byte, high bit set while more follow, at most five bytes.
    fn base128(&mut self) -> Result<u32, String> {
        let mut v: u32 = 0;
        for i in 0..5 {
            let b = self.u8()?;
            if i == 0 && b == 0x80 {
                return Err(NOT_A_FONT.into());
            }
            if v & 0xFE00_0000 != 0 {
                return Err(NOT_A_FONT.into());
            }
            v = (v << 7) | (b & 0x7F) as u32;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(NOT_A_FONT.into())
    }

    /// WOFF2's 255UInt16.
    fn u255(&mut self) -> Result<u16, String> {
        Ok(match self.u8()? {
            253 => self.u16()?,
            254 => self.u8()? as u16 + 506,
            255 => self.u8()? as u16 + 253,
            c => c as u16,
        })
    }
}

fn checksum(b: &[u8]) -> u32 {
    b.chunks(4).fold(0u32, |s, c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        s.wrapping_add(u32::from_be_bytes(w))
    })
}

/// A font file of these tables, laid out as the OpenType specification asks.
pub(super) fn write_sfnt(flavor: u32, tables: &Tables) -> Vec<u8> {
    let n = tables.len() as u16;
    let mut selector = 0u16;
    while n > 0 && (2u32 << selector) <= n as u32 {
        selector += 1;
    }
    let range = if n > 0 { (1u16 << selector) * 16 } else { 0 };
    let mut out = Vec::new();
    out.extend(flavor.to_be_bytes());
    for v in [n, range, selector, (n * 16).saturating_sub(range)] {
        out.extend(v.to_be_bytes());
    }
    let mut offset = 12 + 16 * n as usize;
    let mut body = Vec::new();
    let mut head_at = None;
    for (tag, data) in tables {
        let mut data = data.clone();
        if tag == b"head" && data.len() >= 12 {
            data[8..12].fill(0);
            head_at = Some(offset);
        }
        out.extend(tag);
        out.extend(checksum(&data).to_be_bytes());
        out.extend((offset as u32).to_be_bytes());
        out.extend((data.len() as u32).to_be_bytes());
        let padded = (data.len() + 3) & !3;
        body.extend(&data);
        body.resize(body.len() + padded - data.len(), 0);
        offset += padded;
    }
    out.extend(body);
    if let Some(at) = head_at {
        let adjust = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[at + 8..at + 12].copy_from_slice(&adjust.to_be_bytes());
    }
    out
}
