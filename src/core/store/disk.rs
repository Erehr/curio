//! The store's file primitives: links, copies, renames and removals on disk.

use std::io;
use std::path::{Path, PathBuf};
use crate::core::db::fold_path;
use crate::core::system::filesystem;
use crate::core::system::logging;

/// Whether `dir` (created when missing) is on a filesystem that supports hard links.
pub fn probe_hard_links(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let a = dir.join(format!(".curio-link-probe-{}", fastrand::u32(..)));
    let b = a.with_extension("2");
    let ok = std::fs::write(&a, b"probe").is_ok() && std::fs::hard_link(&a, &b).is_ok();
    let _ = std::fs::remove_file(&b);
    let _ = std::fs::remove_file(&a);
    ok
}

/// Rename a file or directory without ever replacing what is at `dest`. A change of case only is allowed.
#[cfg(windows)]
pub fn rename_no_clobber(src: &Path, dest: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVE_FILE_FLAGS};
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p)?;
    }
    // the raw call needs the `\\?\` form for paths past 260 characters, which std adds by itself
    let w = |p: &Path| -> Vec<u16> {
        // the verbatim form takes no forward slashes
        let s = p.as_os_str().to_string_lossy().replace('/', "\\");
        let long = if s.starts_with(r"\\?\") {
            s.to_string()
        } else if let Some(unc) = s.strip_prefix(r"\\") {
            format!(r"\\?\UNC\{unc}")
        } else {
            format!(r"\\?\{s}")
        };
        std::ffi::OsStr::new(&long).encode_wide().chain(std::iter::once(0)).collect()
    };
    let (s, d) = (w(src), w(dest));
    // without MOVEFILE_REPLACE_EXISTING the call fails when `dest` exists, atomically
    unsafe { MoveFileExW(PCWSTR(s.as_ptr()), PCWSTR(d.as_ptr()), MOVE_FILE_FLAGS(0)) }.map_err(|e| io::Error::from_raw_os_error(e.code().0 & 0xFFFF))
}

/// Rename a file or directory without ever replacing what is at `dest`.
#[cfg(not(windows))]
pub fn rename_no_clobber(src: &Path, dest: &Path) -> io::Result<()> {
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p)?;
    }
    // RENAME_NOREPLACE refuses an existing `dest` in the same step; a filesystem without it falls back to looking first
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        let c = |p: &Path| std::ffi::CString::new(p.as_os_str().as_bytes()).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e));
        let (s, d) = (c(src)?, c(dest)?);
        if unsafe { libc::renameat2(libc::AT_FDCWD, s.as_ptr(), libc::AT_FDCWD, d.as_ptr(), libc::RENAME_NOREPLACE) } == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ENOSYS) | Some(libc::EINVAL) => {}
            // a new case of the same name, on a filesystem that ignores case
            Some(libc::EEXIST) if same_entry(src, dest) => {}
            _ => return Err(err),
        }
    }
    if std::fs::symlink_metadata(dest).is_ok() && !same_entry(src, dest) {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} already exists", dest.display())));
    }
    std::fs::rename(src, dest)
}

/// Whether two paths name one entry, as two cases of a name do where case is ignored.
#[cfg(not(windows))]
fn same_entry(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::symlink_metadata(a), std::fs::symlink_metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
        _ => false,
    }
}

/// A second entry for `src` at `dest`: a hard link when `links` and the filesystem allow, else a full copy. Returns whether it is a link. Never replaces `dest`.
pub fn link_or_copy(src: &Path, dest: &Path, links: bool) -> io::Result<bool> {
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p)?;
    }
    if std::fs::symlink_metadata(dest).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} already exists", dest.display())));
    }
    if links {
        match std::fs::hard_link(src, dest) {
            Ok(()) => return Ok(true),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists || e.kind() == io::ErrorKind::NotFound => return Err(e),
            Err(e) => logging::warn(&format!("couldn't link {}, copying it instead: {e}", dest.display())),
        }
    }
    copy_verified(src, dest)?;
    Ok(false)
}

/// Copy through `<dest>.partial-copy`, check size and content hash, then rename into place. Never replaces `dest`.
pub fn copy_verified(src: &Path, dest: &Path) -> io::Result<()> {
    let tmp = PathBuf::from(format!("{}.partial-copy", dest.display()));
    let res = (|| {
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(src, &tmp)?;
        std::fs::OpenOptions::new().write(true).open(&tmp)?.sync_all()?;
        let (a, b) = (std::fs::metadata(src)?.len(), std::fs::metadata(&tmp)?.len());
        if a != b || filesystem::content_hash(src)? != filesystem::content_hash(&tmp)? {
            return Err(io::Error::other(format!("the copy of {} doesn't match the original", src.display())));
        }
        rename_no_clobber(&tmp, dest)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    match (filesystem::file_key(a), filesystem::file_key(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

pub(super) fn exists(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

/// The name an entry has on disk, which differs from `p`'s only in case on a case-insensitive filesystem.
pub(super) fn name_on_disk(p: &Path) -> Option<String> {
    let want = fold_path(&filesystem::name_of(p));
    std::fs::read_dir(p.parent()?).ok()?.flatten().map(|e| e.file_name().to_string_lossy().to_string()).find(|n| fold_path(n) == want)
}
