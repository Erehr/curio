//! Items written out as JSON and CSV, and sidecar files written beside them.

use crate::core::import::sanitize_name;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::core::db::{Db, DbResult};
use crate::core::model::*;

const EXPORT_PREFIX: &str = "curio-metadata/";
const EXPORT_VERSION: u32 = 1;
pub const EXPORT_FORMAT: &str = "curio-metadata/1";

/// Sidecars are replaced only when their JSON `format` is `curio-metadata/<n>` at or below what this build writes.
fn is_replaceable_sidecar(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else { return false };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return false };
    let Some(fmt) = v.get("format").and_then(|f| f.as_str()) else { return false };
    let Some(ver) = fmt.strip_prefix(EXPORT_PREFIX).and_then(|s| s.parse::<u32>().ok()) else { return false };
    ver <= EXPORT_VERSION
}

pub fn asset_json(db: &Db, a: &Asset, fields: &HashMap<i64, FieldDef>) -> serde_json::Value {
    assets_json(db, std::slice::from_ref(a), fields).pop().unwrap_or_default()
}

/// `asset_json` for many items, reading their tags, folders and field values with one query each.
pub fn assets_json(db: &Db, assets: &[Asset], fields: &HashMap<i64, FieldDef>) -> Vec<serde_json::Value> {
    let ids: Vec<AssetId> = assets.iter().map(|a| a.id).collect();
    let (tags, folders) = tag_and_folder_names(db, &ids);
    let values = values_by_asset(db, &ids);
    let by_hand = names_by_asset(db, "SELECT o.asset_id, f.key FROM field_overrides o JOIN fields f ON f.id = o.field_id WHERE o.asset_id IN ({}) ORDER BY o.asset_id, f.key", &ids);
    assets
        .iter()
        .map(|a| {
            let mut j = one_json(a, fields, tags.get(&a.id).cloned().unwrap_or_default(), folders.get(&a.id).cloned().unwrap_or_default(), values.get(&a.id).cloned().unwrap_or_default());
            if let Some(keys) = by_hand.get(&a.id) {
                j["overrides"] = serde_json::json!(keys);
            }
            if a.parts > 0 {
                let parts = db.members_of(a.id).unwrap_or_default();
                j["parts"] = serde_json::json!(parts.iter().map(|p| serde_json::json!({"name": p.name, "role": p.role, "ext": p.ext, "size": p.size})).collect::<Vec<_>>());
            }
            j
        })
        .collect()
}

/// Tag names and folder names of each item, alphabetical.
pub fn tag_and_folder_names(db: &Db, ids: &[AssetId]) -> (HashMap<AssetId, Vec<String>>, HashMap<AssetId, Vec<String>>) {
    let tags = names_by_asset(db, "SELECT x.asset_id, t.name FROM asset_tags x JOIN tags t ON t.id = x.tag_id WHERE x.asset_id IN ({}) ORDER BY x.asset_id, t.name COLLATE NOCASE", ids);
    let folders = names_by_asset(db, "SELECT x.asset_id, f.name FROM asset_folders x JOIN folders f ON f.id = x.folder_id WHERE x.asset_id IN ({}) ORDER BY x.asset_id, f.name COLLATE NOCASE", ids);
    (tags, folders)
}

fn names_by_asset(db: &Db, sql: &str, ids: &[AssetId]) -> HashMap<AssetId, Vec<String>> {
    let mut out: HashMap<AssetId, Vec<String>> = HashMap::new();
    for chunk in ids.chunks(900) {
        let sql = sql.replace("{}", &vec!["?"; chunk.len()].join(","));
        let Ok(mut st) = db.conn.prepare(&sql) else { continue };
        let Ok(rows) = st.query_map(rusqlite::params_from_iter(chunk.iter()), |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) else { continue };
        for (id, name) in rows.flatten() {
            out.entry(id).or_default().push(name);
        }
    }
    out
}

fn values_by_asset(db: &Db, ids: &[AssetId]) -> HashMap<AssetId, Vec<FieldValue>> {
    let mut out: HashMap<AssetId, Vec<FieldValue>> = HashMap::new();
    for chunk in ids.chunks(900) {
        let sql = format!("SELECT asset_id, field_id, idx, text_value, num_value FROM field_values WHERE asset_id IN ({}) ORDER BY asset_id, field_id, idx", vec!["?"; chunk.len()].join(","));
        let Ok(mut st) = db.conn.prepare(&sql) else { continue };
        let Ok(rows) = st.query_map(rusqlite::params_from_iter(chunk.iter()), |r| Ok((r.get::<_, i64>(0)?, FieldValue { field_id: r.get(1)?, idx: r.get(2)?, text: r.get(3)?, num: r.get(4)? }))) else { continue };
        for (id, v) in rows.flatten() {
            out.entry(id).or_default().push(v);
        }
    }
    out
}

fn one_json(a: &Asset, fields: &HashMap<i64, FieldDef>, tags: Vec<String>, folders: Vec<String>, values: Vec<FieldValue>) -> serde_json::Value {
    let mut fv: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut grouped: HashMap<i64, Vec<FieldValue>> = HashMap::new();
    for v in values {
        grouped.entry(v.field_id).or_default().push(v);
    }
    for (fid, vals) in grouped {
        let Some(def) = fields.get(&fid) else { continue };
        let conv = |v: &FieldValue| -> serde_json::Value {
            if def.ftype.is_numeric() && !matches!(def.ftype, FieldType::Date | FieldType::DateTime) {
                match (&def.ftype, v.num) {
                    (FieldType::Boolean, n) => serde_json::Value::Bool(n.unwrap_or(0.0) != 0.0),
                    (FieldType::Integer, Some(n)) if n.fract() == 0.0 && n.abs() < 9e15 => serde_json::json!(n as i64),
                    (_, n) => serde_json::json!(n),
                }
            } else if def.ftype == FieldType::Json {
                v.text.as_deref().and_then(|t| serde_json::from_str(t).ok()).unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::json!(v.text.clone().or_else(|| v.num.map(|n| n.to_string())))
            }
        };
        let val = if vals.len() == 1 && !def.multi() { conv(&vals[0]) } else { serde_json::Value::Array(vals.iter().map(conv).collect()) };
        fv.insert(def.key.clone(), val);
    }
    serde_json::json!({
        "uid": a.uid,
        "path": a.abs_path,
        "name": a.name,
        "display_name": a.display_name,
        "ext": a.ext,
        "mime": a.mime,
        "kind": a.kind.as_str(),
        "size": a.size,
        "width": a.width,
        "height": a.height,
        "duration": a.duration,
        "created": a.ctime,
        "modified": a.mtime,
        "imported": a.added_at,
        "rating": a.rating,
        "favorite": a.favorite,
        "explicit": a.explicit,
        "annotation": a.annotation,
        "source_url": a.source_url,
        "tags": tags,
        "folders": folders,
        "palette": a.palette,
        "quick_hash": a.quick_hash,
        "content_hash": a.content_hash,
        "status": a.status.label(),
        "fields": fv,
    })
}

/// Export the given assets (or the whole library) as JSON.
pub fn export_json(db: &Db, ids: &[AssetId], dest: &Path) -> DbResult<usize> {
    let fields: HashMap<i64, FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
    let f = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::new(f);
    let defs: Vec<&FieldDef> = fields.values().collect();
    write!(
        w,
        "{{\"format\":\"{EXPORT_FORMAT}\",\"exported_at\":{},\"fields\":{},\"assets\":[",
        now_ms(),
        serde_json::to_string(&defs).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;
    let mut n = 0;
    for chunk in ids.chunks(500) {
        for j in assets_json(db, &db.assets(chunk)?, &fields) {
            if n > 0 {
                w.write_all(b",").map_err(|e| e.to_string())?;
            }
            serde_json::to_writer(&mut w, &j).map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    w.write_all(b"]}").map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())?;
    Ok(n)
}

fn csv_cell(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn export_csv(db: &Db, ids: &[AssetId], dest: &Path) -> DbResult<usize> {
    let fields = db.fields()?;
    let by_id: HashMap<i64, FieldDef> = fields.iter().map(|f| (f.id, f.clone())).collect();
    let f = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::new(f);
    let mut header = vec!["uid", "path", "name", "ext", "size", "width", "height", "rating", "favorite", "explicit", "tags", "folders", "annotation", "source_url"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    header.extend(fields.iter().map(|f| f.key.clone()));
    writeln!(w, "{}", header.iter().map(|h| csv_cell(h)).collect::<Vec<_>>().join(",")).map_err(|e| e.to_string())?;
    let mut n = 0;
    for chunk in ids.chunks(500) {
        let list = db.assets(chunk)?;
        for (a, j) in list.iter().zip(assets_json(db, &list, &by_id)) {
            let mut row = vec![
                a.uid.clone(),
                a.abs_path.clone(),
                a.title().to_string(),
                a.ext.clone(),
                a.size.to_string(),
                a.width.map(|v| v.to_string()).unwrap_or_default(),
                a.height.map(|v| v.to_string()).unwrap_or_default(),
                a.rating.to_string(),
                a.favorite.to_string(),
                a.explicit.to_string(),
                j["tags"].as_array().map(|v| v.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("; ")).unwrap_or_default(),
                j["folders"].as_array().map(|v| v.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("; ")).unwrap_or_default(),
                a.annotation.clone(),
                a.source_url.clone(),
            ];
            for fd in &fields {
                let v = &j["fields"][&fd.key];
                row.push(match v {
                    serde_json::Value::Null => String::new(),
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(a) => a.iter().map(|x| x.as_str().map(|s| s.to_string()).unwrap_or_else(|| x.to_string())).collect::<Vec<_>>().join("; "),
                    other => other.to_string(),
                });
            }
            writeln!(w, "{}", row.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(",")).map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    w.flush().map_err(|e| e.to_string())?;
    Ok(n)
}

/// Write `<file>.curio.json` sidecars. Either next to the originals (explicit user choice) or
/// mirrored into `dest_dir`. Existing sidecars are only replaced if they are ours.
/// Returns how many were written and how many were refused because their location is read-only.
pub fn write_sidecars(db: &Db, ids: &[AssetId], dest_dir: Option<&Path>) -> DbResult<(usize, usize)> {
    let fields: HashMap<i64, FieldDef> = db.fields()?.into_iter().map(|f| (f.id, f)).collect();
    let (mut n, mut refused) = (0, 0);
    for chunk in ids.chunks(200) {
        let list = db.assets(chunk)?;
        for (a, mut j) in list.iter().zip(assets_json(db, &list, &fields)) {
            if dest_dir.is_none() && db.writes_beside(a.root_id).is_err() {
                refused += 1;
                continue;
            }
            let target = match dest_dir {
                Some(d) => d.join(format!("{}-{}.curio.json", a.uid, sanitize_name(&a.name))),
                None => PathBuf::from(format!("{}.curio.json", a.abs_path)),
            };
            if target.exists() && !is_replaceable_sidecar(&target) {
                continue;
            }
            j["format"] = serde_json::json!(EXPORT_FORMAT);
            if std::fs::write(&target, serde_json::to_vec_pretty(&j).unwrap_or_default()).is_ok() {
                n += 1;
            }
        }
    }
    if n == 0 && refused > 0 {
        return Err("No sidecars written: the items are in the library's own storage or in read-only locations.".into());
    }
    Ok((n, refused))
}
