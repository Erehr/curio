//! Whether a still and a clip of the same name are one Live Photo: the clip carries Apple's content identifier, and the still's, when it can be read, is the same.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const KEY: &[u8] = b"com.apple.quicktime.content.identifier";
const MAX_MOOV: u64 = 16 * 1024 * 1024;

/// The children of an atom's payload: type and payload.
fn atoms(buf: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut out = vec![];
    let mut at = 0usize;
    while at + 8 <= buf.len() {
        let size = u32::from_be_bytes(buf[at..at + 4].try_into().unwrap()) as usize;
        let kind: [u8; 4] = buf[at + 4..at + 8].try_into().unwrap();
        let (head, size) = match size {
            0 => (8, buf.len() - at),
            1 if at + 16 <= buf.len() => (16, usize::try_from(u64::from_be_bytes(buf[at + 8..at + 16].try_into().unwrap())).unwrap_or(usize::MAX)),
            n => (8, n),
        };
        // a size is whatever the file says: one past the end, or wrapping around, ends the list
        let Some(end) = at.checked_add(size).filter(|end| size >= head && *end <= buf.len()) else { break };
        out.push((kind, &buf[at + head..end]));
        at = end;
    }
    out
}

/// The clip's `moov` atom, found by walking the file's top-level atoms (it may come last).
fn moov(path: &Path) -> Option<Vec<u8>> {
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let mut at = 0u64;
    while at + 8 <= len {
        f.seek(SeekFrom::Start(at)).ok()?;
        let mut head = [0u8; 16];
        f.read_exact(&mut head[..8]).ok()?;
        let mut size = u32::from_be_bytes(head[..4].try_into().unwrap()) as u64;
        let mut skip = 8;
        if size == 1 {
            f.read_exact(&mut head[8..16]).ok()?;
            size = u64::from_be_bytes(head[8..16].try_into().unwrap());
            skip = 16;
        } else if size == 0 {
            size = len - at;
        }
        if size < skip {
            return None;
        }
        if &head[4..8] == b"moov" {
            let n = (size - skip).min(MAX_MOOV);
            let mut buf = vec![0u8; n as usize];
            f.read_exact(&mut buf).ok()?;
            return Some(buf);
        }
        at = at.checked_add(size)?;
    }
    None
}

/// The content identifier a clip's `moov/meta` keys and items hold: `None` without the key, `Some(None)` with the key but no readable value.
pub fn clip_identifier(path: &Path) -> Option<Option<String>> {
    let moov = moov(path)?;
    if !moov.windows(KEY.len()).any(|w| w == KEY) {
        return None;
    }
    let found = atoms(&moov).into_iter().filter(|(k, _)| k == b"meta").find_map(|(_, meta)| {
        let parts = atoms(meta);
        let keys = parts.iter().find(|(k, _)| k == b"keys")?.1;
        let ilst = parts.iter().find(|(k, _)| k == b"ilst")?.1;
        // version and flags, the count, then each key as size, namespace and name
        let mut at = 8usize;
        let mut index = 0u32;
        let mut wanted = None;
        while at + 8 <= keys.len() {
            let size = u32::from_be_bytes(keys[at..at + 4].try_into().ok()?) as usize;
            let Some(end) = at.checked_add(size).filter(|end| size >= 8 && *end <= keys.len()) else { break };
            index += 1;
            if &keys[at + 8..end] == KEY {
                wanted = Some(index);
            }
            at = end;
        }
        let wanted = wanted?;
        let (_, item) = atoms(ilst).into_iter().find(|(k, _)| u32::from_be_bytes(*k) == wanted)?;
        let (_, data) = atoms(item).into_iter().find(|(k, _)| k == b"data")?;
        let value = data.get(8..)?;
        Some(String::from_utf8_lossy(value).trim_matches('\0').trim().to_string())
    });
    Some(found.filter(|s| !s.is_empty()))
}

/// The content identifier in a still's Apple MakerNote (tag 0x0011), when there is one to read.
pub fn still_identifier(path: &Path) -> Option<String> {
    let mut r = std::io::BufReader::new(std::fs::File::open(path).ok()?);
    let ex = exif::Reader::new().read_from_container(&mut r).ok()?;
    let note = ex.get_field(exif::Tag::MakerNote, exif::In::PRIMARY)?;
    let exif::Value::Undefined(b, _) = &note.value else { return None };
    if !b.starts_with(b"Apple iOS\0") || b.len() < 16 {
        return None;
    }
    // "Apple iOS\0", a version, the byte order, then an IFD whose offsets count from the note's start
    let big = &b[12..14] == b"MM";
    let u16_at = |i: usize| b.get(i..i + 2).map(|x| if big { u16::from_be_bytes([x[0], x[1]]) } else { u16::from_le_bytes([x[0], x[1]]) });
    let u32_at = |i: usize| b.get(i..i + 4).map(|x| if big { u32::from_be_bytes([x[0], x[1], x[2], x[3]]) } else { u32::from_le_bytes([x[0], x[1], x[2], x[3]]) });
    let count = u16_at(14)? as usize;
    for e in 0..count {
        let at = 16 + e * 12;
        if u16_at(at)? != 0x0011 {
            continue;
        }
        let n = u32_at(at + 4)? as usize;
        let off = if n <= 4 { at + 8 } else { u32_at(at + 8)? as usize };
        let s = b.get(off..off.checked_add(n)?)?;
        return Some(String::from_utf8_lossy(s).trim_matches('\0').trim().to_string()).filter(|s| !s.is_empty());
    }
    None
}

/// The pair is a Live Photo: the clip has the key, and its identifier matches the still's when both can be read.
pub fn is_live_photo(still: &Path, clip: &Path) -> bool {
    match clip_identifier(clip) {
        None => false,
        Some(None) => true,
        Some(Some(c)) => still_identifier(still).is_none_or(|s| s.eq_ignore_ascii_case(&c)),
    }
}
