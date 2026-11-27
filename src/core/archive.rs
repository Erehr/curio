//! What is inside ZIP files (CBZ comics, EPUB books, plain ZIP) and 7z files (CB7 comics): the entries in reading order, one entry's bytes, its pictures' sizes, and packing pictures into a new CBZ or ZIP.
//! Archives read lately stay open (`open`), so a reader turning pages never opens the file or reads its index again.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Condvar, LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

/// What one entry may hold before it is refused; the declared size can lie, so reads stop there too.
pub const ENTRY_LIMIT: u64 = 256 << 20;
const PICTURES: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif", "jxl", "tif", "tiff"];
/// How many archives stay open, and for how long unused: an open file can't be deleted outright on Windows.
const KEEP_OPEN: usize = 8;
const IDLE: Duration = Duration::from_secs(30);
/// A 7z is unpacked whole the first time an entry is read, unless what it holds is larger than this.
const UNPACK_LIMIT: u64 = 4 << 30;
/// Room reserved for an entry before reading it: its declared size, which can lie, up to this.
const RESERVE: u64 = 16 << 20;
/// How much of a picture is read for its size; a JPEG whose metadata is longer gets a second, longer look.
const HEAD: u64 = 64 << 10;
const LONG_HEAD: u64 = 2 << 20;

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub picture: bool,
}

/// A picture's size as it shows (a JPEG's stored turn applied), and whether it moves.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Look {
    pub width: u32,
    pub height: u32,
    pub animated: bool,
}

/// `page2` before `page10`: runs of digits compare as numbers, the rest without case.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (m, n) = (take(&mut x), take(&mut y));
                let (mt, nt) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                let o = mt.len().cmp(&nt.len()).then_with(|| mt.cmp(nt));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// Reading order of paths, folder by folder, each name compared naturally: `Chapter 2 (b)/1.jpg` comes after `Chapter 2/9.jpg`, where comparing whole paths would put the space before the slash.
/// A folder's own files come before its subfolders, as a cover beside the chapters' folders is read first.
pub fn reading_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.split('/').peekable(), b.split('/').peekable());
    loop {
        match (x.next(), y.next()) {
            (None, None) => return a.cmp(b),
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(s), Some(t)) => match (x.peek().is_none(), y.peek().is_none()) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => match natural_cmp(s, t) {
                    Ordering::Equal => {}
                    o => return o,
                },
            },
        }
    }
}

pub fn is_picture(name: &str) -> bool {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    PICTURES.contains(&ext.as_str())
}

/// Files a system left beside the real content: macOS forks, thumbnail caches.
pub(crate) fn is_noise(name: &str) -> bool {
    let l = name.to_lowercase();
    l.starts_with("__macosx/") || l.contains("/.") || l.starts_with('.') || l.ends_with("thumbs.db") || l.ends_with(".ds_store")
}

/// The extensions Curio reads inside.
pub const EXTS: &[&str] = &["zip", "cbz", "epub", "7z", "cb7"];

/// A 7z file, by its signature rather than its name.
pub(crate) fn is_7z(path: &Path) -> bool {
    let mut head = [0u8; 6];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok() && head == *b"7z\xBC\xAF\x27\x1C"
}

fn open_7z(path: &Path) -> Result<sevenz_rust::SevenZReader<std::fs::File>, String> {
    sevenz_rust::SevenZReader::open(path, sevenz_rust::Password::empty()).map_err(|e| format!("Not an archive Curio reads: {e}"))
}

/// A file read at positions of its own, so clones read one open file at once from several threads.
#[derive(Clone)]
struct PosFile {
    file: Arc<std::fs::File>,
    pos: u64,
    len: u64,
}

impl Read for PosFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        #[cfg(windows)]
        let n = std::os::windows::fs::FileExt::seek_read(&*self.file, buf, self.pos)?;
        #[cfg(unix)]
        let n = std::os::unix::fs::FileExt::read_at(&*self.file, buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for PosFile {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let at = match to {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::End(n) => self.len as i128 + n as i128,
            SeekFrom::Current(n) => self.pos as i128 + n as i128,
        };
        if at < 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek before the start"));
        }
        self.pos = at as u64;
        Ok(self.pos)
    }
}

/// A 7z unpacked into a folder of its own, entry by entry as they come out; the folder goes with the last holder.
struct Unpack {
    dir: PathBuf,
    state: Mutex<(HashMap<String, PathBuf>, bool)>,
    ready: Condvar,
    stop: AtomicBool,
}

impl Drop for Unpack {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// An archive as read once: its entries in reading order, its ZIP index with the file kept open, and its pictures' sizes once asked for.
pub struct Archive {
    path: PathBuf,
    stamp: (u64, Option<SystemTime>),
    entries: Vec<Entry>,
    zip: Option<zip::ZipArchive<PosFile>>,
    looks: OnceLock<HashMap<String, Look>>,
    unpack: OnceLock<Option<Arc<Unpack>>>,
    used: AtomicU64,
}

impl Drop for Archive {
    fn drop(&mut self) {
        if let Some(Some(u)) = self.unpack.get() {
            u.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);
static OPEN: LazyLock<Mutex<Vec<Arc<Archive>>>> = LazyLock::new(|| {
    // archives nobody reads are closed after a while, by one thread for all of them
    std::thread::spawn(|| loop {
        std::thread::sleep(IDLE / 2);
        let now = STARTED.elapsed().as_secs();
        OPEN.lock().unwrap_or_else(|p| p.into_inner()).retain(|a| now.saturating_sub(a.used.load(std::sync::atomic::Ordering::Relaxed)) < IDLE.as_secs());
    });
    Mutex::new(Vec::new())
});

/// The archive at `path`, kept open from an earlier read while the file is unchanged.
pub fn open(path: &Path) -> Result<Arc<Archive>, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let stamp = (meta.len(), meta.modified().ok());
    let now = STARTED.elapsed().as_secs();
    {
        let mut open = OPEN.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(i) = open.iter().position(|a| a.path == path && a.stamp == stamp) {
            let a = open.remove(i);
            a.used.store(now, std::sync::atomic::Ordering::Relaxed);
            open.push(a.clone());
            return Ok(a);
        }
    }
    let a = Arc::new(Archive::read(path, stamp, now)?);
    let mut open = OPEN.lock().unwrap_or_else(|p| p.into_inner());
    open.retain(|o| o.path != path);
    open.push(a.clone());
    if open.len() > KEEP_OPEN {
        open.remove(0);
    }
    Ok(a)
}

impl Archive {
    fn read(path: &Path, stamp: (u64, Option<SystemTime>), now: u64) -> Result<Archive, String> {
        let mut a = Archive { path: path.to_path_buf(), stamp, entries: Vec::new(), zip: None, looks: OnceLock::new(), unpack: OnceLock::new(), used: AtomicU64::new(now) };
        if is_7z(path) {
            let z = open_7z(path)?;
            a.entries = z.archive().files.iter().filter(|e| !e.is_directory && e.has_stream && !is_noise(&e.name)).map(|e| Entry { name: e.name.clone(), size: e.size, picture: is_picture(&e.name) }).collect();
        } else {
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut z = zip::ZipArchive::new(PosFile { file: Arc::new(file), pos: 0, len: stamp.0 }).map_err(|e| format!("Not an archive Curio reads: {e}"))?;
            for i in 0..z.len() {
                let Ok(e) = z.by_index_raw(i) else { continue };
                if e.is_dir() || is_noise(e.name()) {
                    continue;
                }
                a.entries.push(Entry { name: e.name().to_string(), size: e.size(), picture: is_picture(e.name()) });
            }
            a.zip = Some(z);
        }
        a.entries.sort_by(|x, y| reading_cmp(&x.name, &y.name));
        Ok(a)
    }

    /// Every file inside, in reading order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// One entry's bytes; from several threads at once for a ZIP.
    pub fn read_entry(&self, name: &str) -> Result<Vec<u8>, String> {
        if let Some(z) = &self.zip {
            return read_zip(&mut z.clone(), name, ENTRY_LIMIT);
        }
        let size = self.entries.iter().find(|e| e.name == name).map(|e| e.size).ok_or_else(|| format!("The archive has no {name}"))?;
        if size > ENTRY_LIMIT {
            return Err(format!("{name} is larger than {} MB", ENTRY_LIMIT >> 20));
        }
        if let Some(u) = self.unpacked() {
            if let Some(file) = wait_for(u, name) {
                return std::fs::read(file).map_err(|e| e.to_string());
            }
        }
        read_7z_entry(&self.path, name, size)
    }

    /// The pictures' sizes, read from their first bytes: every picture of a ZIP, and those of a 7z already unpacked.
    pub fn looks(&self) -> HashMap<String, Look> {
        if let Some(l) = self.looks.get() {
            return l.clone();
        }
        let Some(z) = &self.zip else {
            let Some(Some(u)) = self.unpack.get() else { return HashMap::new() };
            let state = u.state.lock().unwrap_or_else(|p| p.into_inner());
            let looks: HashMap<String, Look> = state.0.iter().filter(|(n, _)| is_picture(n)).filter_map(|(n, p)| Some((n.clone(), head_look(&std::fs::read(p).ok()?)?))).collect();
            // kept once every entry is out, as later ones would be missing
            if state.1 {
                let _ = self.looks.set(looks.clone());
            }
            return looks;
        };
        let pictures: Vec<&Entry> = self.entries.iter().filter(|e| e.picture).collect();
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
        let chunk = pictures.len().div_ceil(threads).max(1);
        let looks = std::thread::scope(|s| {
            let jobs: Vec<_> = pictures
                .chunks(chunk)
                .map(|part| {
                    let mut z = z.clone();
                    s.spawn(move || part.iter().filter_map(|e| Some((e.name.clone(), zip_look(&mut z, e)?))).collect::<Vec<_>>())
                })
                .collect();
            jobs.into_iter().flat_map(|j| j.join().unwrap_or_default()).collect::<HashMap<_, _>>()
        });
        self.looks.get_or_init(|| looks).clone()
    }

    /// A 7z's unpacking, begun the first time it is needed; none when it holds too much.
    fn unpacked(&self) -> Option<&Arc<Unpack>> {
        self.unpack
            .get_or_init(|| {
                let total = self.entries.iter().try_fold(0u64, |t, e| t.checked_add(e.size));
                if total.is_none_or(|t| t > UNPACK_LIMIT) {
                    return None;
                }
                let dir = unpack_root().join(format!("{:016x}", unpack_key(&self.path, self.stamp)));
                std::fs::create_dir_all(&dir).ok()?;
                let u = Arc::new(Unpack { dir, state: Mutex::new((HashMap::new(), false)), ready: Condvar::new(), stop: AtomicBool::new(false) });
                let (worker, path) = (u.clone(), self.path.clone());
                std::thread::spawn(move || unpack_all(&path, &worker));
                Some(u)
            })
            .as_ref()
    }
}

fn read_zip(z: &mut zip::ZipArchive<PosFile>, name: &str, limit: u64) -> Result<Vec<u8>, String> {
    let e = z.by_name(name).map_err(|_| format!("The archive has no {name}"))?;
    if e.size() > ENTRY_LIMIT {
        return Err(format!("{name} is larger than {} MB", ENTRY_LIMIT >> 20));
    }
    let mut buf = Vec::with_capacity(e.size().min(limit).min(RESERVE) as usize);
    e.take(limit.min(ENTRY_LIMIT + 1)).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.len() as u64 > ENTRY_LIMIT {
        return Err(format!("{name} is larger than {} MB", ENTRY_LIMIT >> 20));
    }
    Ok(buf)
}

fn zip_look(z: &mut zip::ZipArchive<PosFile>, e: &Entry) -> Option<Look> {
    let head = read_zip(z, &e.name, HEAD).ok()?;
    head_look(&head).or_else(|| (e.size > HEAD).then(|| read_zip(z, &e.name, LONG_HEAD).ok().and_then(|b| head_look(&b))).flatten())
}

/// A picture's size and motion from its first bytes, without decoding it.
pub fn head_look(b: &[u8]) -> Option<Look> {
    let s = imagesize::blob_size(b).ok()?;
    let (w, h) = (s.width as u32, s.height as u32);
    if w == 0 || h == 0 {
        return None;
    }
    let turned = b.starts_with(&[0xFF, 0xD8])
        && exif::Reader::new().read_from_container(&mut std::io::Cursor::new(b)).ok().and_then(|x| x.get_field(exif::Tag::Orientation, exif::In::PRIMARY).and_then(|f| f.value.get_uint(0))).is_some_and(|o| (5..=8).contains(&o));
    let (width, height) = if turned { (h, w) } else { (w, h) };
    Some(Look { width, height, animated: animated(b) })
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Whether a GIF, PNG, WebP or AVIF holds more than one frame, as its header says.
fn animated(b: &[u8]) -> bool {
    if b.starts_with(b"GIF8") {
        return find(b, b"NETSCAPE2.0").is_some();
    }
    if b.starts_with(b"\x89PNG") {
        return matches!((find(b, b"acTL"), find(b, b"IDAT")), (Some(a), Some(i)) if a < i);
    }
    if b.len() > 20 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" && &b[12..16] == b"VP8X" {
        return b[20] & 0x02 != 0;
    }
    b.len() > 12 && &b[4..8] == b"ftyp" && find(&b[8..b.len().min(64)], b"avis").is_some()
}

fn unpack_root() -> PathBuf {
    static ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
        let root = std::env::temp_dir().join("curio-unpacked");
        // left behind by a run that ended without tidying up
        if let Ok(rd) = std::fs::read_dir(&root) {
            let day = SystemTime::now() - Duration::from_secs(86_400);
            for e in rd.flatten() {
                if e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < day) {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        root
    });
    ROOT.clone()
}

fn unpack_key(path: &Path, stamp: (u64, Option<SystemTime>)) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (path, stamp.0, stamp.1, std::process::id()).hash(&mut h);
    h.finish()
}

fn unpack_all(path: &Path, u: &Unpack) {
    let mut n = 0usize;
    // what was written, since declared sizes can lie
    let mut written = 0u64;
    let result = open_7z(path).map(|mut z| {
        z.for_each_entries(|e, r| {
            if u.stop.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(false);
            }
            if e.is_directory || !e.has_stream || e.size > ENTRY_LIMIT || is_noise(&e.name) {
                return Ok(true);
            }
            n += 1;
            let file = u.dir.join(n.to_string());
            let mut buf = Vec::with_capacity(e.size.min(RESERVE) as usize);
            r.take(ENTRY_LIMIT + 1).read_to_end(&mut buf)?;
            written += buf.len() as u64;
            if written > UNPACK_LIMIT {
                return Ok(false);
            }
            if buf.len() as u64 <= ENTRY_LIMIT && std::fs::write(&file, &buf).is_ok() {
                u.state.lock().unwrap_or_else(|p| p.into_inner()).0.insert(e.name.clone(), file);
                u.ready.notify_all();
            }
            Ok(true)
        })
    });
    if let Err(e) = result {
        crate::core::system::logging::warn(&format!("Unpacking {} stopped: {e}", path.display()));
    }
    u.state.lock().unwrap_or_else(|p| p.into_inner()).1 = true;
    u.ready.notify_all();
}

/// Where an entry of an unpacking 7z lands, once it does; none when unpacking ended without it.
fn wait_for(u: &Unpack, name: &str) -> Option<PathBuf> {
    let mut state = u.state.lock().unwrap_or_else(|p| p.into_inner());
    loop {
        if let Some(p) = state.0.get(name) {
            return Some(p.clone());
        }
        if state.1 {
            return None;
        }
        let (s, timeout) = u.ready.wait_timeout(state, Duration::from_secs(120)).unwrap_or_else(|p| p.into_inner());
        if timeout.timed_out() {
            return None;
        }
        state = s;
    }
}

/// One entry of a 7z read on its own: a solid archive decodes what comes before it, and reading stops once it is found.
fn read_7z_entry(path: &Path, name: &str, size: u64) -> Result<Vec<u8>, String> {
    let mut z = open_7z(path)?;
    let mut found = None;
    z.for_each_entries(|e, r| {
        if e.name != name {
            return Ok(true);
        }
        let mut buf = Vec::with_capacity(size.min(RESERVE) as usize);
        r.take(ENTRY_LIMIT + 1).read_to_end(&mut buf)?;
        found = Some(buf);
        Ok(false)
    })
    .map_err(|e| e.to_string())?;
    found.filter(|b| b.len() as u64 <= ENTRY_LIMIT).ok_or_else(|| format!("Couldn't read {name}"))
}

/// Every file inside, in reading order.
pub fn list(path: &Path) -> Result<Vec<Entry>, String> {
    Ok(open(path)?.entries.clone())
}

/// One entry's bytes.
pub fn read(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    open(path)?.read_entry(name)
}

/// The first picture inside in reading order, for the archive's thumbnail.
pub fn first_picture(path: &Path) -> Option<Vec<u8>> {
    let a = open(path).ok()?;
    let first = a.entries.iter().find(|e| e.picture && e.size <= 64 << 20)?;
    a.read_entry(&first.name).ok()
}

/// Packs files into a new ZIP at `out`, each under its name, in the order given; pictures are stored as they are, as compressing them gains nothing.
pub fn create(out: &Path, files: &[(String, std::path::PathBuf)]) -> Result<(), String> {
    let partial = out.with_extension("partial");
    let result = (|| {
        let mut w = zip::ZipWriter::new(std::fs::File::create(&partial).map_err(|e| e.to_string())?);
        for (name, path) in files {
            let method = if is_picture(name) { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };
            let opts = zip::write::SimpleFileOptions::default().compression_method(method).large_file(std::fs::metadata(path).map(|m| m.len() > u32::MAX as u64).unwrap_or(false));
            w.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
            std::io::copy(&mut f, &mut w).map_err(|e| e.to_string())?;
        }
        w.finish().map_err(|e| e.to_string())?.flush().map_err(|e| e.to_string())
    })();
    match result {
        Ok(()) => std::fs::rename(&partial, out).map_err(|e| e.to_string()),
        Err(e) => {
            let _ = std::fs::remove_file(&partial);
            Err(e)
        }
    }
}
