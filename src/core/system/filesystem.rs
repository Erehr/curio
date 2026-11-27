//! Platform-independent filesystem layer. Every filesystem access of the core goes through
//! here so path rules, identity and "is this storage reachable" logic live in one place.
//! Nothing in this module modifies or deletes user files; the only writes are into the
//! library's own folders (managed store, caches, exports chosen by the user).

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::core::consts::QUICK_HASH_CHUNK;

#[derive(Clone, Debug, PartialEq)]
pub struct FileStat {
    pub size: u64,
    /// milliseconds since epoch
    pub mtime: Option<i64>,
    pub ctime: Option<i64>,
    /// Filesystem identity (volume + file index / device + inode) when available.
    pub file_key: Option<String>,
    pub is_dir: bool,
}

fn to_ms(t: std::io::Result<std::time::SystemTime>) -> Option<i64> {
    t.ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
}

/// Writes a file whole or not at all: beside it first, then in its place, so another process starting at the same time never reads half of it.
pub fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        fs::create_dir_all(d)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = std::path::PathBuf::from(tmp);
    fs::write(&tmp, bytes).and_then(|_| fs::rename(&tmp, path)).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// What a directory listing says of a file, without opening it (Windows lists sizes and times with the names); `file_key`, which opens the file, is left for `file_key`.
pub fn listed(md: &fs::Metadata) -> FileStat {
    FileStat { size: md.len(), mtime: to_ms(md.modified()), ctime: to_ms(md.created()), file_key: None, is_dir: md.is_dir() }
}

pub fn stat(path: &Path) -> std::io::Result<FileStat> {
    let md = fs::metadata(path)?;
    Ok(FileStat {
        size: md.len(),
        mtime: to_ms(md.modified()),
        ctime: to_ms(md.created()),
        file_key: file_key(path),
        is_dir: md.is_dir(),
    })
}

/// Stable identity of a file on its filesystem (survives rename/move within a volume).
pub fn file_key(path: &Path) -> Option<String> {
    match file_id::get_file_id(path) {
        Ok(id) => Some(format!("{id:?}")),
        Err(_) => None,
    }
}

/// Canonical absolute path as a String with native separators (no `\\?\` prefix on Windows).
pub fn canonical(path: &Path) -> PathBuf {
    let p = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    strip_verbatim(p)
}

/// `canonical` for a path that may not exist yet: its nearest existing folder is resolved and the rest added back.
pub fn canonical_existing_part(path: &Path) -> PathBuf {
    let mut rest = vec![];
    let mut at = path;
    loop {
        if let Ok(c) = fs::canonicalize(at) {
            return strip_verbatim(rest.iter().rev().fold(c, |p: PathBuf, n| p.join(n)));
        }
        match (at.parent(), at.file_name()) {
            (Some(up), Some(n)) => {
                rest.push(n.to_os_string());
                at = up;
            }
            _ => return path.to_path_buf(),
        }
    }
}

pub fn strip_verbatim(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest.to_string());
        }
    }
    p
}

/// Relative path stored in the database: always '/'-separated so libraries move between OSes.
pub fn rel_to_db(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The database may come from someone else: a part that isn't a plain name (`..`, a drive, a path) becomes `_`, so the result never leaves `root`.
pub fn db_to_abs(root: &str, rel: &str) -> PathBuf {
    let mut p = PathBuf::from(root);
    for part in rel.split('/').filter(|s| !s.is_empty()) {
        let mut parts = Path::new(part).components();
        match (parts.next(), parts.next()) {
            (Some(std::path::Component::Normal(_)), None) => p.push(part),
            _ => p.push("_"),
        }
    }
    p
}

/// Is the storage that holds `root` reachable right now? A missing root directory on a
/// removable or network drive means "offline", not "files deleted".
pub fn root_online(root: &Path) -> bool {
    fs::read_dir(root).is_ok()
}

/// Volume-ish prefix of a path (drive letter, UNC share or mount point guess).
pub fn volume_of(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        if s.starts_with(r"\\") {
            let parts: Vec<&str> = s.trim_start_matches('\\').splitn(3, '\\').collect();
            if parts.len() >= 2 {
                return format!(r"\\{}\{}", parts[0], parts[1]);
            }
        }
        if s.len() >= 2 && s.as_bytes()[1] == b':' {
            return s[..2].to_uppercase();
        }
    }
    #[cfg(not(windows))]
    {
        for prefix in ["/media/", "/mnt/", "/run/media/", "/Volumes/"] {
            if let Some(rest) = s.strip_prefix(prefix) {
                let mut it = rest.split('/');
                let first = it.next().unwrap_or("");
                if prefix == "/run/media/" {
                    let second = it.next().unwrap_or("");
                    return format!("{prefix}{first}/{second}");
                }
                return format!("{prefix}{first}");
            }
        }
        return "/".into();
    }
    #[allow(unreachable_code)]
    s
}

/// What kind of storage holds `path`: `network`, `removable` or `read_only_media`, or nothing for a local disk.
/// Only a hint for the user; it never decides what Curio may write.
#[cfg(windows)]
pub fn volume_hint(path: &Path) -> Option<&'static str> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumeInformationW};
    // winbase.h DRIVE_* and FILE_READ_ONLY_VOLUME
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;
    const FILE_READ_ONLY_VOLUME: u32 = 0x80000;
    let vol = volume_of(path);
    if vol.starts_with(r"\\") {
        return Some("network");
    }
    if vol.len() != 2 {
        return None;
    }
    let wide: Vec<u16> = std::ffi::OsStr::new(&format!("{vol}\\")).encode_wide().chain(Some(0)).collect();
    let kind = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
    match kind {
        DRIVE_REMOTE => return Some("network"),
        DRIVE_CDROM => return Some("read_only_media"),
        _ => {}
    }
    let mut flags = 0u32;
    if unsafe { GetVolumeInformationW(PCWSTR(wide.as_ptr()), None, None, None, Some(&mut flags), None) }.is_ok() && flags & FILE_READ_ONLY_VOLUME != 0 {
        return Some("read_only_media");
    }
    (kind == DRIVE_REMOVABLE).then_some("removable")
}

/// Bytes free for this user on the drive holding `path`.
#[cfg(windows)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), None, None) }.ok()?;
    Some(free)
}

#[cfg(unix)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
        return None;
    }
    Some(s.f_bavail as u64 * s.f_frsize as u64)
}

#[cfg(not(any(windows, unix)))]
pub fn free_space(_path: &Path) -> Option<u64> {
    None
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn volume_hint(path: &Path) -> Option<&'static str> {
    const NETWORK: [&str; 9] = ["nfs", "nfs4", "cifs", "smb3", "smbfs", "fuse.sshfs", "fuse.rclone", "9p", "afs"];
    let mounts = fs::read_to_string("/proc/self/mounts").ok()?;
    // the longest mount point that contains the path is the one holding it
    let (point, fstype, opts) = mounts
        .lines()
        .filter_map(|l| {
            let mut f = l.split(' ');
            let (_, point, fstype, opts) = (f.next()?, f.next()?, f.next()?, f.next()?);
            Some((point.replace(r"\040", " ").replace(r"\011", "\t"), fstype.to_string(), opts.to_string()))
        })
        .filter(|(point, _, _)| path.starts_with(point))
        .max_by_key(|(point, _, _)| point.len())?;
    if NETWORK.contains(&fstype.as_str()) {
        Some("network")
    } else if opts.split(',').any(|o| o == "ro") {
        Some("read_only_media")
    } else if point.starts_with("/media/") || point.starts_with("/run/media/") {
        Some("removable")
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
pub fn volume_hint(path: &Path) -> Option<&'static str> {
    use std::os::unix::ffi::OsStrExt;
    const NETWORK: [&str; 4] = ["smbfs", "nfs", "afpfs", "webdav"];
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let text = |b: &[libc::c_char]| String::from_utf8_lossy(&b.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect::<Vec<u8>>()).into_owned();
    let flags = st.f_flags as u32;
    if flags & libc::MNT_LOCAL as u32 == 0 || NETWORK.contains(&text(&st.f_fstypename).as_str()) {
        Some("network")
    } else if flags & libc::MNT_RDONLY as u32 != 0 {
        Some("read_only_media")
    } else if text(&st.f_mntonname).starts_with("/Volumes/") {
        // the startup disk is mounted at / and /System/Volumes/Data, everything else under /Volumes
        Some("removable")
    } else {
        None
    }
}

/// Families of the fonts installed on this computer, sorted, without duplicates.
#[cfg(windows)]
pub fn font_families() -> Vec<String> {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::Graphics::Gdi::{EnumFontFamiliesExW, GetDC, ReleaseDC, DEFAULT_CHARSET, LOGFONTW, TEXTMETRICW};
    unsafe extern "system" fn each(lf: *const LOGFONTW, _: *const TEXTMETRICW, _: u32, out: LPARAM) -> i32 {
        let names = unsafe { &mut *(out.0 as *mut Vec<String>) };
        let face = unsafe { &(*lf).lfFaceName };
        let len = face.iter().position(|c| *c == 0).unwrap_or(face.len());
        names.push(String::from_utf16_lossy(&face[..len]));
        1
    }
    let mut names: Vec<String> = Vec::new();
    unsafe {
        let dc = GetDC(None);
        let lf = LOGFONTW { lfCharSet: DEFAULT_CHARSET, ..Default::default() };
        EnumFontFamiliesExW(dc, &lf, Some(each), LPARAM(&mut names as *mut Vec<String> as isize), 0);
        ReleaseDC(None, dc);
    }
    // names starting with @ are the vertical variants of East Asian fonts
    tidy_families(names.into_iter().filter(|n| !n.starts_with('@')).collect())
}

#[cfg(target_os = "macos")]
pub fn font_families() -> Vec<String> {
    use objc2_foundation::{NSArray, NSString};
    let names = unsafe { objc2_core_text::CTFontManagerCopyAvailableFontFamilyNames() };
    // a CFArray of CFStrings is the same object as an NSArray of NSStrings
    let names: &NSArray<NSString> = unsafe { &*(objc2_core_foundation::CFRetained::as_ptr(&names).as_ptr() as *const NSArray<NSString>) };
    tidy_families(names.iter().map(|n| n.to_string()).collect())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn font_families() -> Vec<String> {
    let out = std::process::Command::new("fc-list").args([":", "family"]).output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    // a font with names in several languages lists them all, comma separated: the first is enough
    tidy_families(text.lines().filter_map(|l| l.split(',').next()).map(|s| s.trim().to_string()).collect())
}

fn tidy_families(mut names: Vec<String>) -> Vec<String> {
    names.retain(|n| !n.is_empty());
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

pub fn ext_of(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn stem_of(path: &Path) -> String {
    path.file_stem().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()
}

pub fn name_of(path: &Path) -> String {
    path.file_name().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()
}

/// Endings of Curio's own unfinished files, in the store and beside linked files.
pub const TEMP_ENDINGS: &[&str] = &[".partial", ".partial-copy", ".replacing", ".relinking", ".turning"];

/// One of Curio's own unfinished files or probes, never an item.
pub fn is_temp_name(name: &str) -> bool {
    TEMP_ENDINGS.iter().any(|t| name.ends_with(t)) || name.starts_with(".curio-")
}

/// Hidden or system entries we never index.
pub fn is_ignored_name(name: &str) -> bool {
    name.starts_with('.')
        || name.eq_ignore_ascii_case("thumbs.db")
        || name.eq_ignore_ascii_case("desktop.ini")
        || name == "$RECYCLE.BIN"
        || name == "System Volume Information"
        || name.ends_with(".curio")
        || name.ends_with(".tmp")
        || name.ends_with(".part")
        || name.ends_with(".crdownload")
        || name.starts_with("~$")
        || is_temp_name(name)
}

/// Quick identity hash: size + first and last 64 KiB, the whole file up to 128 KiB. Cheap enough for every new file and strong enough to recognise a moved file; not used to claim exact duplicates.
pub fn quick_hash(path: &Path, size: u64) -> std::io::Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = xxhash_rust::xxh3::Xxh3::new();
    h.update(&size.to_le_bytes());
    let mut buf = vec![0u8; QUICK_HASH_CHUNK];
    let n = read_full(&mut f, &mut buf)?;
    h.update(&buf[..n]);
    if size > (QUICK_HASH_CHUNK * 2) as u64 {
        f.seek(SeekFrom::End(-(QUICK_HASH_CHUNK as i64)))?;
    }
    if size > QUICK_HASH_CHUNK as u64 {
        let n = read_full(&mut f, &mut buf)?;
        h.update(&buf[..n]);
    }
    Ok(format!("{:016x}", h.digest()))
}

fn read_full(f: &mut fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        let n = f.read(&mut buf[total..])?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Ok(total)
}

/// Full content hash (BLAKE3) for exact-duplicate detection and identity recovery.
pub fn content_hash(path: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_hex().to_string())
}

/// Copy a file into the library without ever overwriting: picks a free name.
pub fn copy_no_clobber(src: &Path, dest: &Path) -> std::io::Result<PathBuf> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut target = dest.to_path_buf();
    let mut n = 1;
    while target.exists() {
        let stem = stem_of(dest);
        let ext = ext_of(dest);
        let name = if ext.is_empty() { format!("{stem} ({n})") } else { format!("{stem} ({n}).{ext}") };
        target = dest.with_file_name(name);
        n += 1;
    }
    let tmp = target.with_extension("partial-copy");
    if let Err(e) = fs::copy(src, &tmp).and_then(|_| fs::rename(&tmp, &target)) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(target)
}

/// Quietly to the Recycle Bin: no prompts or error dialogs, and undoable from Explorer.
/// (FOF_SILENT | FOF_NOCONFIRMATION | FOF_ALLOWUNDO | FOF_NOERRORUI | FOFX_RECYCLEONDELETE)
#[cfg(windows)]
const RECYCLE_FLAGS: u32 = 0x0004 | 0x0010 | 0x0040 | 0x0400 | 0x0008_0000;

pub const NO_RECYCLE_BIN: &str = "No Recycle Bin for this location, so the file was kept";

/// Cancels a delete Windows would carry out permanently instead of recycling.
#[cfg(windows)]
#[windows_core::implement(windows::Win32::UI::Shell::IFileOperationProgressSink)]
struct RecycleOnly(std::sync::Arc<std::sync::atomic::AtomicBool>);

#[cfg(windows)]
#[allow(non_snake_case)]
impl windows::Win32::UI::Shell::IFileOperationProgressSink_Impl for RecycleOnly_Impl {
    fn StartOperations(&self) -> windows_core::Result<()> {
        Ok(())
    }
    fn FinishOperations(&self, _: windows_core::HRESULT) -> windows_core::Result<()> {
        Ok(())
    }
    fn PreRenameItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostRenameItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR, _: windows_core::HRESULT, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>) -> windows_core::Result<()> {
        Ok(())
    }
    fn PreMoveItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostMoveItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR, _: windows_core::HRESULT, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>) -> windows_core::Result<()> {
        Ok(())
    }
    fn PreCopyItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostCopyItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR, _: windows_core::HRESULT, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>) -> windows_core::Result<()> {
        Ok(())
    }
    fn PreDeleteItem(&self, flags: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>) -> windows_core::Result<()> {
        if flags & windows::Win32::UI::Shell::TSF_DELETE_RECYCLE_IF_POSSIBLE.0 as u32 != 0 {
            return Ok(());
        }
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
        Err(windows::Win32::UI::Shell::COPYENGINE_E_USER_CANCELLED.into())
    }
    fn PostDeleteItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: windows_core::HRESULT, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>) -> windows_core::Result<()> {
        Ok(())
    }
    fn PreNewItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostNewItem(&self, _: u32, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>, _: &windows_core::PCWSTR, _: &windows_core::PCWSTR, _: u32, _: windows_core::HRESULT, _: windows_core::Ref<windows::Win32::UI::Shell::IShellItem>) -> windows_core::Result<()> {
        Ok(())
    }
    fn UpdateProgress(&self, _: u32, _: u32) -> windows_core::Result<()> {
        Ok(())
    }
    fn ResetTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }
    fn PauseTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }
    fn ResumeTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }
}

/// Send a file to the Recycle Bin. Each step fails with its own message; a person or program
/// cancelling counts as a failure, so the caller knows the file is still there. A location
/// without a Recycle Bin fails with [`NO_RECYCLE_BIN`] instead of deleting the file for good.
#[cfg(windows)]
pub fn to_recycle_bin(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{FileOperation, IFileOperation, IFileOperationProgressSink, IShellItem, SHCreateItemFromParsingName, FILEOPERATION_FLAGS};

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    // on its own thread: this needs a single-threaded apartment whatever the caller's is
    std::thread::Builder::new()
        .name("recycle".into())
        .spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let run = || -> Result<(), String> {
                let op: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_ALL).map_err(|_| "Failed to create FileOperation instance".to_string())?;
                op.SetOperationFlags(FILEOPERATION_FLAGS(RECYCLE_FLAGS)).map_err(|_| "Failed to set operation flags".to_string())?;
                let refused = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let sink: IFileOperationProgressSink = RecycleOnly(refused.clone()).into();
                op.Advise(&sink).map_err(|_| "Failed to watch the delete operation".to_string())?;
                let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).map_err(|_| "Failed to parse path".to_string())?;
                op.DeleteItem(&item, None).map_err(|_| "Failed to enqueue DeleteItem command".to_string())?;
                let done = op.PerformOperations();
                if refused.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(NO_RECYCLE_BIN.into());
                }
                done.map_err(|_| "Failed to perform delete operation".to_string())?;
                if op.GetAnyOperationsAborted().map_err(|_| "Failed to check operation status".to_string())?.as_bool() {
                    return Err("Operation was aborted".into());
                }
                Ok(())
            };
            let r = run();
            CoUninitialize();
            r
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "the file operation crashed".to_string())?
}

/// The Trash, from which Finder can put the file back; a volume without one fails rather than deleting the file for good.
#[cfg(target_os = "macos")]
pub fn to_recycle_bin(path: &Path) -> Result<(), String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    NSFileManager::defaultManager().trashItemAtURL_resultingItemURL_error(&url, None).map_err(|e| e.localizedDescription().to_string())
}

/// The desktop's trash, through the standard tool.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn to_recycle_bin(path: &Path) -> Result<(), String> {
    let status = std::process::Command::new("gio").arg("trash").arg(path).status().map_err(|e| format!("could not reach the trash: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("could not move the file to the trash".into())
    }
}

/// Reveal a file in the platform's file manager.
/// Show a file selected in its folder (SHOpenFolderAndSelectItems
/// with one selected item, on a worker thread). A missing file is an error, not a silent no-op.
pub fn reveal(path: &Path) -> Result<(), String> {
    opener::reveal(path).map_err(|_| format!("Can't open {}", path.display()))
}

/// Open with the default app (folders and files take separate paths,
/// failure reports "Failed to open path").
pub fn open_default(path: &Path) -> Result<(), String> {
    opener::open(path).map_err(|_| "Failed to open path".to_string())
}

/// Find an executable on PATH (used for optional tools such as ffmpeg).
pub fn which(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(&exe);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
