//! Files past the JSON body's cap: sent once as a request's body to the process on the port, kept in a spool folder of the app's data, and taken by `item.add.fromBytes` with the id, in whichever process holds the library.
//! A call carries only the id, never a path: any argument may come from a client, and is forwarded as it came.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{ApiError, ApiResult};

/// The largest upload.
pub const MAX_UPLOAD: u64 = 2 << 30;
/// Free space an upload leaves on its drive.
const MARGIN: u64 = 512 << 20;
/// How long an upload waits to be taken.
const KEEP_MS: i64 = 10 * 60 * 1000;

pub fn spool(data_dir: &Path) -> PathBuf {
    data_dir.join("spool")
}

/// Who sent an upload, and when; beside the file as `<id>.json`.
#[derive(Serialize, Deserialize)]
struct Ticket {
    token: String,
    name: String,
    size: u64,
    at: i64,
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Writes the body to the spool as it arrives; `length` is the request's own when it gave one.
pub fn receive(dir: &Path, body: &mut dyn Read, length: Option<u64>, token: &str, name: &str, max: u64) -> ApiResult {
    let too_large = || ApiError::new("too_large", format!("An upload is at most {} MB", max >> 20));
    if length.is_some_and(|n| n > max) {
        return Err(too_large());
    }
    std::fs::create_dir_all(dir).map_err(|e| ApiError::new("failed", e.to_string()))?;
    sweep(dir, false);
    if let (Some(n), Some(free)) = (length, crate::core::system::filesystem::free_space(dir)) {
        if n.saturating_add(MARGIN) > free {
            return Err(ApiError::new("too_large", "There isn't room for it on the drive Curio keeps its data on"));
        }
    }
    let id = format!("{:032x}", fastrand::u128(..));
    let part = dir.join(format!("{id}.part"));
    let written = std::fs::File::create(&part).and_then(|mut f| std::io::copy(&mut body.take(max + 1), &mut f));
    let fail = |e: ApiError| {
        let _ = std::fs::remove_file(&part);
        Err(e)
    };
    let size = match written {
        Ok(n) => n,
        Err(e) => return fail(ApiError::new("failed", format!("The upload broke off: {e}"))),
    };
    if size > max {
        return fail(too_large());
    }
    if size == 0 {
        return fail(ApiError::new("bad_request", "The upload is empty"));
    }
    if length.is_some_and(|n| n != size) {
        return fail(ApiError::new("failed", "The upload broke off"));
    }
    let ticket = Ticket { token: token.to_string(), name: crate::core::import::sanitize_name(name), size, at: crate::core::model::now_ms() };
    let saved = std::fs::write(dir.join(format!("{id}.json")), serde_json::to_vec(&ticket).unwrap_or_default()).and_then(|_| std::fs::rename(&part, dir.join(&id)));
    if let Err(e) = saved {
        let _ = std::fs::remove_file(dir.join(format!("{id}.json")));
        return fail(ApiError::new("failed", e.to_string()));
    }
    Ok(json!({ "upload": id, "size": size }))
}

/// Checks an id before any work: its form, and that the caller has a token to match it with.
pub fn check(id: &str, token: Option<&str>) -> Result<(), ApiError> {
    if !valid_id(id) {
        return Err(ApiError::new("bad_request", "`upload` is an id from POST /api/v1/upload"));
    }
    if token.is_none() {
        return Err(ApiError::new("not_allowed", "Uploads are for web API tokens"));
    }
    Ok(())
}

/// Takes the upload for the token that sent it, once: the file is renamed out of reach of a second claim, and is the caller's to move or delete.
pub fn claim(dir: &Path, id: &str, token: Option<&str>) -> Result<PathBuf, String> {
    check(id, token).map_err(|e| e.message)?;
    let gone = || format!("No upload {id}: it was used, it expired, or another token sent it");
    let ticket_path = dir.join(format!("{id}.json"));
    let ticket: Ticket = std::fs::read(&ticket_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or_else(gone)?;
    if Some(ticket.token.as_str()) != token {
        return Err(gone());
    }
    if crate::core::model::now_ms() - ticket.at > KEEP_MS {
        let _ = std::fs::remove_file(dir.join(id));
        let _ = std::fs::remove_file(&ticket_path);
        return Err(gone());
    }
    let taken = dir.join(format!("{id}.taking-{:08x}", fastrand::u32(..)));
    std::fs::rename(dir.join(id), &taken).map_err(|_| gone())?;
    let _ = std::fs::remove_file(&ticket_path);
    Ok(taken)
}

/// Removes what has waited longer than an upload is kept: files never taken, their tickets, broken writes and claims whose import ended without them.
pub fn sweep(dir: &Path, all: bool) {
    let Ok(list) = std::fs::read_dir(dir) else { return };
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_millis(KEEP_MS as u64);
    for e in list.flatten() {
        let old = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < cutoff);
        if all || old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}
