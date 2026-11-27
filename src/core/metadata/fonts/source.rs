//! Where a font's bytes come from: its file, read in the parts its tables need.

use std::io::{Read, Seek, SeekFrom};

pub(crate) trait Source {
    fn size(&self) -> u64;
    fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, String>;
}

/// A font file on disk.
pub(crate) struct FileSource {
    file: std::fs::File,
    size: u64,
}

impl FileSource {
    pub(crate) fn open(path: &std::path::Path) -> Result<FileSource, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        Ok(FileSource { file, size })
    }
}

impl Source for FileSource {
    fn size(&self) -> u64 {
        self.size
    }

    fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, String> {
        if offset.checked_add(len as u64).is_none_or(|end| end > self.size) {
            return Err(ENDS_EARLY.into());
        }
        self.file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; len];
        self.file.read_exact(&mut buf).map_err(|_| ENDS_EARLY.to_string())?;
        Ok(buf)
    }
}

pub(super) const ENDS_EARLY: &str = "The file ends early";

pub(super) fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

pub(super) fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

pub(super) fn u24_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 3)?;
    Some(((s[0] as u32) << 16) | ((s[1] as u32) << 8) | s[2] as u32)
}

pub(super) fn tag_at(b: &[u8], at: usize) -> Option<[u8; 4]> {
    b.get(at..at + 4)?.try_into().ok()
}

/// A 16.16 fixed-point number.
pub(super) fn fixed_at(b: &[u8], at: usize) -> Option<f32> {
    Some(u32_at(b, at)? as i32 as f32 / 65536.0)
}
