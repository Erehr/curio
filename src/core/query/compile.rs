//! An expression compiled to a WHERE fragment with its parameters.

use rusqlite::types::Value;
use crate::core::consts::{COLOR_ACCURATE_DE2000, COLOR_DE2000_PER_ACCURACY, COLOR_LOOSEST_DE2000, COLOR_SIMILAR_DE2000};
use crate::core::model::{FieldDef, FieldType};
use crate::core::query::fields::Builtin;
use crate::core::query::parser::word_value;
use crate::core::query::Op;
use crate::core::query::Val;
use crate::core::query::parser::parse;
use crate::core::query::fields::builtin_for;
use crate::core::query::fields::FieldCatalog;
use crate::core::query::Expr;

const DAY_MS: f64 = 86_400_000.0;

pub(super) const EMPTY_LIST: &str = "A list needs at least one value";

fn local_midnight_ms(y: i32, m: u32, d: u32) -> Option<f64> {
    use chrono::TimeZone;
    let dt = chrono::NaiveDate::from_ymd_opt(y, m, d)?.and_hms_opt(0, 0, 0)?;
    chrono::Local.from_local_datetime(&dt).earliest().map(|d| d.timestamp_millis() as f64)
}

/// `2024` or `2024-05` as the whole year or month, start inclusive and end exclusive.
fn period_ms(s: &str) -> Option<(f64, f64)> {
    let (y, m) = match s.split_once(['-', '/', '.']) {
        Some((y, m)) if m.len() <= 2 => (y.parse::<i32>().ok()?, Some(m.parse::<u32>().ok()?)),
        None => (s.parse::<i32>().ok()?, None),
        _ => return None,
    };
    if !(1000..=9999).contains(&y) {
        return None;
    }
    match m {
        Some(m) if (1..=12).contains(&m) => {
            let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            Some((local_midnight_ms(y, m, 1)?, local_midnight_ms(ny, nm, 1)?))
        }
        Some(_) => None,
        None => Some((local_midnight_ms(y, 1, 1)?, local_midnight_ms(y + 1, 1, 1)?)),
    }
}

#[derive(Debug)]
pub struct Compiled {
    pub sql: String,
    pub params: Vec<Value>,
}

pub fn compile(expr: &Expr, cat: &FieldCatalog) -> Result<Compiled, String> {
    let mut c = Compiler { cat, params: Vec::new() };
    let sql = c.expr(expr)?;
    Ok(Compiled { sql, params: c.params })
}

/// The custom fields a query names, by key: those it compares, and the place fields `NEAR` and `INSIDE` read.
pub fn fields_in(input: &str, cat: &FieldCatalog) -> Vec<String> {
    fn walk(e: &Expr, cat: &FieldCatalog, out: &mut Vec<String>) {
        match e {
            Expr::And(v) | Expr::Or(v) => v.iter().for_each(|x| walk(x, cat, out)),
            Expr::Not(x) => walk(x, cat, out),
            Expr::Cmp { field, .. } if builtin_for(field).is_none() => {
                if let Some(f) = cat.custom(field) {
                    out.push(f.key.clone());
                }
            }
            Expr::Near { .. } | Expr::Inside { .. } => out.extend(["gps_lat".to_string(), "gps_lon".to_string()]),
            Expr::With(v) => v.iter().for_each(|x| walk(x, cat, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(e) = parse(input, &|n| cat.is_field(n)) {
        walk(&e, cat, &mut out);
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|k| seen.insert(k.clone()));
    out
}

/// Parse + compile in one step.
pub fn compile_str(input: &str, cat: &FieldCatalog) -> Result<Compiled, String> {
    let e = parse(input, &|n| cat.is_field(n))?;
    compile(&e, cat)
}

struct Compiler<'a> {
    cat: &'a FieldCatalog,
    params: Vec<Value>,
}

fn unit_mult_size(u: &Option<String>) -> f64 {
    match u.as_deref() {
        Some("kb") => 1024.0,
        Some("mb") => 1024.0 * 1024.0,
        Some("gb") => 1024.0 * 1024.0 * 1024.0,
        Some("tb") => 1024.0f64.powi(4),
        _ => 1.0,
    }
}

fn unit_mult_duration(u: &Option<String>) -> f64 {
    match u.as_deref() {
        Some("m") | Some("min") => 60.0,
        Some("h") | Some("hr") => 3600.0,
        Some("ms") => 0.001,
        _ => 1.0,
    }
}

fn unit_one(_: &Option<String>) -> f64 {
    1.0
}

/// Milliseconds since epoch at local midnight of a YYYY-MM-DD (or full datetime) string.
pub fn parse_date_ms(s: &str) -> Option<f64> {
    use chrono::NaiveDateTime;
    if let Some(d) = day_of(s) {
        return local_ms(d.and_hms_opt(0, 0, 0)?);
    }
    let s = s.trim();
    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return local_ms(dt);
        }
    }
    None
}

/// The day a value names on its own: today, yesterday or a date without a time.
fn day_of(s: &str) -> Option<chrono::NaiveDate> {
    use chrono::NaiveDate;
    let s = s.trim();
    let today = chrono::Local::now().date_naive();
    if s.eq_ignore_ascii_case("today") {
        return Some(today);
    }
    if s.eq_ignore_ascii_case("yesterday") {
        return today.pred_opt();
    }
    ["%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d"].iter().find_map(|f| NaiveDate::parse_from_str(s, f).ok())
}

/// A local time as milliseconds: the earlier of a time the clocks pass twice, and for one they skip, the hour after.
fn local_ms(dt: chrono::NaiveDateTime) -> Option<f64> {
    use chrono::TimeZone;
    let at = |dt| chrono::Local.from_local_datetime(&dt).earliest();
    at(dt).or_else(|| at(dt + chrono::Duration::hours(1))).map(|d| d.timestamp_millis() as f64)
}

fn duration_ms(v: &Val) -> Option<f64> {
    match v {
        Val::Num(n, u) => Some(
            n * match u.as_deref() {
                Some("h") | Some("hr") => 3_600_000.0,
                Some("m") | Some("min") => 60_000.0,
                Some("s") | Some("sec") => 1000.0,
                Some("w") => 7.0 * DAY_MS,
                Some("y") => 365.0 * DAY_MS,
                _ => DAY_MS,
            },
        ),
        _ => None,
    }
}

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim_start_matches('#');
    if !h.is_ascii() || h.len() != 6 {
        return named_color(s);
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r, g, b])
}

fn named_color(s: &str) -> Option<[u8; 3]> {
    Some(match s.to_lowercase().as_str() {
        "red" => [229, 57, 53],
        "orange" => [251, 140, 0],
        "yellow" => [253, 216, 53],
        "green" => [67, 160, 71],
        "aqua" | "cyan" => [0, 172, 193],
        "blue" => [30, 136, 229],
        "purple" => [142, 36, 170],
        "pink" => [236, 64, 122],
        "black" => [0, 0, 0],
        "white" => [255, 255, 255],
        "brown" => [109, 76, 65],
        _ => return None,
    })
}

type Mult = fn(&Option<String>) -> f64;

impl<'a> Compiler<'a> {
    fn p(&mut self, v: Value) -> String {
        self.params.push(v);
        "?".into()
    }
    fn ptext(&mut self, s: &str) -> String {
        self.p(Value::Text(s.to_string()))
    }
    fn pnum(&mut self, n: f64) -> String {
        self.p(Value::Real(n))
    }

    fn expr(&mut self, e: &Expr) -> Result<String, String> {
        Ok(match e {
            Expr::True => "1".into(),
            Expr::And(v) => {
                let mut parts = Vec::new();
                for x in v {
                    parts.push(self.expr(x)?);
                }
                format!("({})", parts.join(" AND "))
            }
            Expr::Or(v) => {
                let mut parts = Vec::new();
                for x in v {
                    parts.push(self.expr(x)?);
                }
                format!("({})", parts.join(" OR "))
            }
            Expr::Not(x) => format!("(NOT {})", self.expr(x)?),
            Expr::Text(t) => self.text(t),
            Expr::Near { lat, lon, km } => {
                let (lat_id, lon_id) = self.gps_ids()?;
                // a box first, which idx_fv_num narrows, then distance on a tangent plane
                let dlat = km / 110.574;
                let scale = (lat.to_radians().cos() * 111.320).max(1e-6);
                let dlon = km / scale;
                let (west, east) = (lon - dlon, lon + dlon);
                let p_lat = self.pnum(*lat);
                let p_lon = self.pnum(*lon);
                let p_la_lo = self.pnum(lat - dlat);
                let p_la_hi = self.pnum(lat + dlat);
                let lon_box = if dlon >= 180.0 {
                    "1".to_string()
                } else if west < -180.0 {
                    let (pw, pe) = (self.pnum(west + 360.0), self.pnum(east));
                    format!("(lo.num_value >= {pw} OR lo.num_value <= {pe})")
                } else if east > 180.0 {
                    let (pw, pe) = (self.pnum(west), self.pnum(east - 360.0));
                    format!("(lo.num_value >= {pw} OR lo.num_value <= {pe})")
                } else {
                    let (pw, pe) = (self.pnum(west), self.pnum(east));
                    format!("lo.num_value BETWEEN {pw} AND {pe}")
                };
                let p_scale1 = self.pnum(scale);
                let p_scale2 = self.pnum(scale);
                let p_km2 = self.pnum(km * km);
                format!(
                    "a.id IN (SELECT asset_id FROM (SELECT la.asset_id, (la.num_value - {p_lat}) * 110.574 AS dy, (lo.num_value - {p_lon}) AS dlo FROM field_values la JOIN field_values lo ON lo.asset_id = la.asset_id AND lo.field_id = {lon_id} WHERE la.field_id = {lat_id} AND la.num_value BETWEEN {p_la_lo} AND {p_la_hi} AND {lon_box}) WHERE dy * dy + ((CASE WHEN dlo > 180 THEN dlo - 360 WHEN dlo < -180 THEN dlo + 360 ELSE dlo END) * {p_scale1}) * ((CASE WHEN dlo > 180 THEN dlo - 360 WHEN dlo < -180 THEN dlo + 360 ELSE dlo END) * {p_scale2}) <= {p_km2})"
                )
            }
            Expr::Inside { south, west, north, east } => {
                let (lat_id, lon_id) = self.gps_ids()?;
                let (ps, pn) = (self.pnum(*south), self.pnum(*north));
                let (pw, pe) = (self.pnum(*west), self.pnum(*east));
                let lon = if west <= east { format!("lo.num_value BETWEEN {pw} AND {pe}") } else { format!("(lo.num_value >= {pw} OR lo.num_value <= {pe})") };
                format!("a.id IN (SELECT la.asset_id FROM field_values la JOIN field_values lo ON lo.asset_id = la.asset_id AND lo.field_id = {lon_id} WHERE la.field_id = {lat_id} AND la.num_value BETWEEN {ps} AND {pn} AND {lon})")
            }
            Expr::With(all) => self.with(all)?,
            Expr::Cmp { field, op, value } => {
                if let Some(b) = builtin_for(field) {
                    self.builtin(b, op, value)?
                } else if let Some(f) = self.cat.custom(field).cloned() {
                    if !f.indexed {
                        return Err(crate::core::db::not_indexed(&f.key));
                    }
                    self.custom(&f, op, value)?
                } else {
                    return Err(format!("Unknown field '{field}'"));
                }
            }
        })
    }

    fn text(&mut self, t: &str) -> String {
        // Full-text search over name, annotation, url, tags and searchable custom text fields.
        let cleaned: String = t.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { ' ' }).collect();
        let terms: Vec<String> = cleaned.split_whitespace().map(|w| format!("\"{w}\"*")).collect();
        if terms.is_empty() {
            let pat = format!("%{}%", escape_like(t));
            let a = self.ptext(&pat);
            return format!("(a.name LIKE {a} ESCAPE '\\')");
        }
        let m = self.ptext(&terms.join(" "));
        format!("a.id IN (SELECT rowid FROM asset_fts WHERE asset_fts MATCH {m})")
    }

    fn string_cond(&mut self, col: &str, op: &Op, v: &Val) -> Result<String, String> {
        let s = v.as_text();
        Ok(match op {
            Op::Eq => format!("{col} = {} COLLATE NOCASE", self.ptext(&s)),
            Op::Ne => format!("COALESCE({col},'') <> {} COLLATE NOCASE", self.ptext(&s)),
            Op::Contains | Op::Similar => {
                let p = self.ptext(&format!("%{}%", escape_like(&s)));
                format!("{col} LIKE {p} ESCAPE '\\'")
            }
            Op::NotContains => {
                let p = self.ptext(&format!("%{}%", escape_like(&s)));
                format!("COALESCE({col},'') NOT LIKE {p} ESCAPE '\\'")
            }
            Op::Starts => {
                let p = self.ptext(&format!("{}%", escape_like(&s)));
                format!("{col} LIKE {p} ESCAPE '\\'")
            }
            Op::Ends => {
                let p = self.ptext(&format!("%{}", escape_like(&s)));
                format!("{col} LIKE {p} ESCAPE '\\'")
            }
            Op::Matches => {
                // a pattern that doesn't compile would quietly match nothing
                regex::RegexBuilder::new(&s).case_insensitive(true).build().map_err(|e| format!("'{s}' is not a valid pattern: {}", e.to_string().lines().last().unwrap_or("")))?;
                format!("{col} REGEXP {}", self.ptext(&s))
            }
            Op::Empty => format!("COALESCE({col},'') = ''"),
            Op::NotEmpty => format!("COALESCE({col},'') <> ''"),
            Op::In | Op::NotIn => {
                let items = v.items();
                if items.is_empty() {
                    return Err(EMPTY_LIST.into());
                }
                let mut ph = Vec::new();
                for x in &items {
                    ph.push(self.ptext(&x.as_text()));
                }
                let not = if *op == Op::NotIn { "NOT " } else { "" };
                format!("{col} COLLATE NOCASE {not}IN ({})", ph.join(","))
            }
            Op::Lt => format!("{col} < {}", self.ptext(&s)),
            Op::Le => format!("{col} <= {}", self.ptext(&s)),
            Op::Gt => format!("{col} > {}", self.ptext(&s)),
            Op::Ge => format!("{col} >= {}", self.ptext(&s)),
            _ => return Err(format!("Operator {op:?} doesn't apply to text")),
        })
    }

    /// For columns stored strictly lowercase with BINARY indexes (`ext`, `kind`, `content_hash`).
    fn string_cond_lower(&mut self, col: &str, op: &Op, v: &Val) -> Result<String, String> {
        match op {
            Op::Eq => Ok(format!("{col} = {}", self.ptext(&v.as_text().to_lowercase()))),
            Op::Ne => Ok(format!("COALESCE({col},'') <> {}", self.ptext(&v.as_text().to_lowercase()))),
            Op::In | Op::NotIn => {
                let items = v.items();
                if items.is_empty() {
                    return Err(EMPTY_LIST.into());
                }
                let mut ph = Vec::new();
                for x in &items {
                    ph.push(self.ptext(&x.as_text().to_lowercase()));
                }
                let not = if *op == Op::NotIn { "NOT " } else { "" };
                Ok(format!("{col} {not}IN ({})", ph.join(",")))
            }
            _ => self.string_cond(col, op, v),
        }
    }

    /// Exact `= 1` / `= 0` for boolean flag columns so SQLite uses partial indexes (`idx_assets_favorite`, `idx_assets_explicit`).
    fn bool_cond(&mut self, col: &str, op: &Op, v: &Val) -> Result<String, String> {
        match op {
            Op::Eq | Op::Similar | Op::Ne => {
                let b = match v {
                    Val::Num(n, None) => *n != 0.0,
                    _ => parse_bool(&v.as_text()).map_err(|_| format!("'{}' is not true/false", v.as_text()))?,
                };
                let want = if *op == Op::Ne { !b } else { b };
                Ok(format!("{col} = {}", if want { 1 } else { 0 }))
            }
            _ => self.num_cond(col, op, v, unit_one),
        }
    }

    fn num_of(&self, v: &Val, mult: Mult) -> Result<f64, String> {
        match v {
            Val::Num(n, u) => Ok(n * mult(u)),
            Val::Str(s) => {
                if let Ok(b) = parse_bool(s) {
                    return Ok(if b { 1.0 } else { 0.0 });
                }
                match word_value(s) {
                    Val::Num(n, u) => Ok(n * mult(&u)),
                    _ => Err(format!("'{s}' is not a number")),
                }
            }
            _ => Err("Expected a number".into()),
        }
    }

    fn num_cond(&mut self, col: &str, op: &Op, v: &Val, mult: Mult) -> Result<String, String> {
        Ok(match op {
            Op::Eq | Op::Similar => {
                let n = self.num_of(v, mult)?;
                // tolerate float noise, e.g. cfg = 1 vs 1.0000001
                let eps = 1e-9_f64.max(n.abs() * 1e-9);
                let lo = self.pnum(n - eps);
                let hi = self.pnum(n + eps);
                format!("({col} BETWEEN {lo} AND {hi})")
            }
            Op::Ne => {
                let n = self.num_of(v, mult)?;
                let p = self.pnum(n);
                format!("({col} IS NULL OR {col} <> {p})")
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                let n = self.num_of(v, mult)?;
                let sym = match op {
                    Op::Lt => "<",
                    Op::Le => "<=",
                    Op::Gt => ">",
                    _ => ">=",
                };
                let p = self.pnum(n);
                format!("{col} {sym} {p}")
            }
            Op::Between => {
                let (a, b) = match v {
                    Val::Pair(a, b) => (self.num_of(a, mult)?, self.num_of(b, mult)?),
                    _ => return Err("BETWEEN needs two values".into()),
                };
                let (a, b) = if a <= b { (a, b) } else { (b, a) };
                let pa = self.pnum(a);
                let pb = self.pnum(b);
                format!("({col} BETWEEN {pa} AND {pb})")
            }
            Op::In | Op::NotIn => {
                if v.items().is_empty() {
                    return Err(EMPTY_LIST.into());
                }
                let mut ph = Vec::new();
                for x in v.items() {
                    let n = self.num_of(&x, mult)?;
                    ph.push(self.pnum(n));
                }
                let not = if *op == Op::NotIn { "NOT " } else { "" };
                format!("{col} {not}IN ({})", ph.join(","))
            }
            Op::Empty => format!("{col} IS NULL"),
            Op::NotEmpty => format!("{col} IS NOT NULL"),
            _ => return Err(format!("Operator {op:?} doesn't apply to numbers")),
        })
    }

    /// The span a date value names: a day, a month or a year; a relative value like `7d` is a single instant.
    fn date_span(&self, v: &Val) -> Result<(f64, f64), String> {
        match v {
            Val::Str(s) => {
                if let Some(p) = period_ms(s.trim()) {
                    return Ok(p);
                }
                // a day runs to the next midnight, 23 or 25 hours away when the clocks change
                if let Some(d) = day_of(s) {
                    let start = local_ms(d.and_hms_opt(0, 0, 0).ok_or("Not a date")?);
                    let end = d.succ_opt().and_then(|n| local_ms(n.and_hms_opt(0, 0, 0)?));
                    return start.zip(end).ok_or(format!("'{s}' is not a date (use YYYY-MM-DD)"));
                }
                parse_date_ms(s).map(|d| (d, d + DAY_MS)).ok_or(format!("'{s}' is not a date (use YYYY-MM-DD)"))
            }
            Val::Num(n, None) if n.fract() == 0.0 => period_ms(&(*n as i64).to_string()).ok_or_else(|| format!("'{n}' is not a year")),
            Val::Num(_, Some(_)) => {
                let t = crate::core::model::now_ms() as f64 - duration_ms(v).unwrap_or(0.0);
                Ok((t, t))
            }
            _ => Err("Expected a date".into()),
        }
    }

    fn date_cond(&mut self, col: &str, op: &Op, v: &Val) -> Result<String, String> {
        // "before X" includes all of X, "on X" is the whole day, month or year.
        if *op == Op::Within {
            let d = duration_ms(v).ok_or("WITHIN needs a duration like 7d, 12h, 2w")?;
            let since = crate::core::model::now_ms() as f64 - d;
            let p = self.pnum(since);
            return Ok(format!("{col} >= {p}"));
        }
        if matches!(op, Op::Empty | Op::NotEmpty) {
            return Ok(format!("{col} IS {}NULL", if *op == Op::NotEmpty { "NOT " } else { "" }));
        }
        if *op == Op::Between {
            let (a, b) = match v {
                Val::Pair(a, b) => (self.date_span(a)?, self.date_span(b)?),
                _ => return Err("BETWEEN needs two dates".into()),
            };
            let (a, b) = if a.0 <= b.0 { (a, b) } else { (b, a) };
            let pa = self.pnum(a.0);
            let pb = self.pnum(b.1.max(a.1));
            return Ok(format!("({col} >= {pa} AND {col} < {pb})"));
        }
        let (start, end) = self.date_span(v)?;
        Ok(match op {
            Op::Eq | Op::Similar => {
                let a = self.pnum(start);
                let b = self.pnum(end);
                format!("({col} >= {a} AND {col} < {b})")
            }
            Op::Ne => {
                let a = self.pnum(start);
                let b = self.pnum(end);
                format!("({col} IS NULL OR {col} < {a} OR {col} >= {b})")
            }
            Op::Lt => format!("{col} < {}", self.pnum(start)),
            Op::Le => format!("{col} < {}", self.pnum(end)),
            Op::Gt => format!("{col} >= {}", self.pnum(end)),
            Op::Ge => format!("{col} >= {}", self.pnum(start)),
            _ => return Err(format!("Operator {op:?} doesn't apply to dates")),
        })
    }

    fn set_one(&mut self, table: &str, key: &str, names: &str, name: &str, how: &str, neg: bool) -> String {
        self.set_one_by(table, key, names, "name", name, how, neg)
    }

    /// An item in a tag or folder whose `col` matches. Positive forms use uncorrelated `IN` so SQLite can drive from the tag/folder index instead of scanning `assets`.
    fn set_one_by(&mut self, table: &str, key: &str, names: &str, col: &str, name: &str, how: &str, neg: bool) -> String {
        let (cmp, val) = match how {
            "contains" => ("LIKE", format!("%{}%", escape_like(name))),
            "starts" => ("LIKE", format!("{}%", escape_like(name))),
            _ => ("=", name.to_string()),
        };
        let p = self.ptext(&val);
        let tail = if cmp == "LIKE" { " ESCAPE '\\'" } else { " COLLATE NOCASE" };
        if neg {
            format!("NOT EXISTS (SELECT 1 FROM {table} x JOIN {names} n ON n.id = x.{key} WHERE x.asset_id = a.id AND n.{col} {cmp} {p}{tail})")
        } else {
            format!("a.id IN (SELECT x.asset_id FROM {table} x JOIN {names} n ON n.id = x.{key} WHERE n.{col} {cmp} {p}{tail})")
        }
    }

    /// Set membership against tags or folders.
    fn set_cond(&mut self, table: &str, key: &str, names: &str, op: &Op, v: &Val) -> Result<String, String> {
        Ok(match op {
            Op::Eq | Op::Similar => self.set_one(table, key, names, &v.as_text(), "eq", false),
            Op::Contains => self.set_one(table, key, names, &v.as_text(), "contains", false),
            Op::Starts => self.set_one(table, key, names, &v.as_text(), "starts", false),
            Op::Ne => self.set_one(table, key, names, &v.as_text(), "eq", true),
            Op::NotContains => self.set_one(table, key, names, &v.as_text(), "contains", true),
            Op::In | Op::HasAll | Op::HasNone | Op::NotIn => {
                let items = v.items();
                if items.is_empty() {
                    return Err(EMPTY_LIST.into());
                }
                match op {
                    Op::In => {
                        let ph: Vec<String> = items.iter().map(|x| self.ptext(&x.as_text())).collect();
                        format!("a.id IN (SELECT x.asset_id FROM {table} x JOIN {names} n ON n.id = x.{key} WHERE n.name COLLATE NOCASE IN ({}))", ph.join(","))
                    }
                    Op::HasAll => {
                        let parts: Vec<String> = items.iter().map(|x| self.set_one(table, key, names, &x.as_text(), "eq", false)).collect();
                        format!("({})", parts.join(" AND "))
                    }
                    _ => {
                        let ph: Vec<String> = items.iter().map(|x| self.ptext(&x.as_text())).collect();
                        format!("NOT EXISTS (SELECT 1 FROM {table} x JOIN {names} n ON n.id = x.{key} WHERE x.asset_id = a.id AND n.name COLLATE NOCASE IN ({}))", ph.join(","))
                    }
                }
            }
            Op::Empty => format!("NOT EXISTS (SELECT 1 FROM {table} x WHERE x.asset_id = a.id)"),
            Op::NotEmpty => format!("EXISTS (SELECT 1 FROM {table} x WHERE x.asset_id = a.id)"),
            _ => return Err(format!("Operator {op:?} doesn't apply to {names}")),
        })
    }

    /// In a folder of that colour (a CSS colour as the folder keeps it, `var(--accent)` for the theme's); a coloured folder's subfolders count only with that colour themselves.
    fn folder_color(&mut self, op: &Op, v: &Val) -> Result<String, String> {
        let one = |c: &mut Self, x: &Val, neg: bool| c.set_one_by("asset_folders", "folder_id", "folders", "color", &x.as_text(), "eq", neg);
        Ok(match op {
            Op::Eq | Op::Similar => one(self, v, false),
            Op::Ne => one(self, v, true),
            Op::In | Op::NotIn => {
                let items = v.items();
                if items.is_empty() {
                    return Err(EMPTY_LIST.into());
                }
                let ph: Vec<String> = items.iter().map(|x| self.ptext(&x.as_text())).collect();
                if *op == Op::In {
                    format!("a.id IN (SELECT x.asset_id FROM asset_folders x JOIN folders n ON n.id = x.folder_id WHERE n.color COLLATE NOCASE IN ({}))", ph.join(","))
                } else {
                    format!("NOT EXISTS (SELECT 1 FROM asset_folders x JOIN folders n ON n.id = x.folder_id WHERE x.asset_id = a.id AND n.color COLLATE NOCASE IN ({}))", ph.join(","))
                }
            }
            Op::Empty => "NOT EXISTS (SELECT 1 FROM asset_folders x JOIN folders n ON n.id = x.folder_id WHERE x.asset_id = a.id AND COALESCE(n.color, '') <> '')".into(),
            Op::NotEmpty => "a.id IN (SELECT x.asset_id FROM asset_folders x JOIN folders n ON n.id = x.folder_id WHERE COALESCE(n.color, '') <> '')".into(),
            _ => return Err(format!("Operator {op:?} doesn't apply to folder_color")),
        })
    }

    fn builtin(&mut self, b: Builtin, op: &Op, v: &Val) -> Result<String, String> {
        Ok(match b {
            Builtin::FileName => self.string_cond("a.name", op, v)?,
            Builtin::Name => self.string_cond("COALESCE(NULLIF(a.display_name,''), a.name)", op, v)?,
            Builtin::Ext => {
                let v2 = match v {
                    Val::Str(s) => Val::Str(s.trim_start_matches('.').to_string()),
                    Val::List(l) => {
                        Val::List(l.iter().map(|x| Val::Str(x.as_text().trim_start_matches('.').to_string())).collect())
                    }
                    o => o.clone(),
                };
                let op2 = if *op == Op::Similar { Op::Eq } else { op.clone() };
                self.string_cond_lower("a.ext", &op2, &v2)?
            }
            Builtin::Kind => {
                let op2 = if *op == Op::Similar { Op::Eq } else { op.clone() };
                self.string_cond_lower("a.kind", &op2, v)?
            }
            Builtin::Mime => self.string_cond("a.mime", op, v)?,
            Builtin::Annotation => self.string_cond("a.annotation", op, v)?,
            Builtin::Url => self.string_cond("a.source_url", op, v)?,
            Builtin::Path => self.string_cond("a.rel_path", op, v)?,
            Builtin::Hash => self.string_cond_lower("a.content_hash", op, v)?,
            Builtin::Id => match op {
                Op::Eq | Op::Similar => format!("a.uid = uid_bytes({})", self.ptext(&v.as_text())),
                Op::Ne => format!("a.uid <> uid_bytes({})", self.ptext(&v.as_text())),
                Op::In | Op::NotIn => {
                    let items = v.items();
                    if items.is_empty() {
                        return Err(EMPTY_LIST.into());
                    }
                    let ph: Vec<String> = items.iter().map(|x| format!("uid_bytes({})", self.ptext(&x.as_text()))).collect();
                    let not = if *op == Op::NotIn { "NOT " } else { "" };
                    format!("a.uid {not}IN ({})", ph.join(","))
                }
                _ => self.string_cond("uid_text(a.uid)", op, v)?,
            },
            Builtin::Size => self.num_cond("a.size", op, v, unit_mult_size)?,
            Builtin::Width => self.num_cond("a.width", op, v, unit_one)?,
            Builtin::Height => self.num_cond("a.height", op, v, unit_one)?,
            Builtin::Aspect => {
                if *op == Op::Empty {
                    return Ok("(a.width IS NULL OR a.height IS NULL OR a.width <= 0 OR a.height <= 0)".into());
                }
                if *op == Op::NotEmpty {
                    return Ok("(a.width > 0 AND a.height > 0)".into());
                }
                let v2 = match v {
                    Val::Str(s) if s.contains('/') => {
                        let mut it = s.split('/');
                        let a: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(1.0);
                        let b: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(1.0);
                        Val::Num(a / b.max(1e-9), None)
                    }
                    o => o.clone(),
                };
                if *op == Op::Ne {
                    let n = self.num_of(&v2, unit_one)?;
                    let lo = self.pnum(n - crate::core::consts::RATIO_TOLERANCE);
                    let hi = self.pnum(n + crate::core::consts::RATIO_TOLERANCE);
                    return Ok(format!("(a.width IS NULL OR a.height IS NULL OR a.width <= 0 OR a.height <= 0 OR NOT (a.aspect BETWEEN {lo} AND {hi}))"));
                }
                let cond = if *op == Op::Eq || *op == Op::Similar {
                    let n = self.num_of(&v2, unit_one)?;
                    let lo = self.pnum(n - crate::core::consts::RATIO_TOLERANCE);
                    let hi = self.pnum(n + crate::core::consts::RATIO_TOLERANCE);
                    format!("(a.aspect BETWEEN {lo} AND {hi})")
                } else {
                    self.num_cond("a.aspect", op, &v2, unit_one)?
                };
                // an item without dimensions has no shape, so it never matches a comparison
                format!("(a.width > 0 AND a.height > 0 AND {cond})")
            }
            Builtin::Duration => self.num_cond("a.duration", op, v, unit_mult_duration)?,
            Builtin::Animated => self.num_cond("COALESCE(a.animated, 0)", op, v, unit_one)?,
            Builtin::Favorite => self.bool_cond("a.favorite", op, v)?,
            Builtin::Explicit => self.bool_cond("a.explicit", op, v)?,
            Builtin::TagCount => self.num_cond("(SELECT count(*) FROM asset_tags tc WHERE tc.asset_id = a.id)", op, v, unit_one)?,
            Builtin::Parts => self.num_cond("(SELECT count(*) FROM assets pc WHERE pc.part_of = a.id)", op, v, unit_one)?,
            // the roles of the files grouped into an item: `part = motion`, `part != raw`, `part IS EMPTY`
            Builtin::Part => {
                let (neg, inner) = match op {
                    Op::Empty => (true, "1".to_string()),
                    Op::NotEmpty => (false, "1".to_string()),
                    Op::Ne => (true, self.string_cond("m.part_role", &Op::Eq, v)?),
                    Op::NotIn => (true, self.string_cond("m.part_role", &Op::In, v)?),
                    Op::NotContains => (true, self.string_cond("m.part_role", &Op::Contains, v)?),
                    o => (false, self.string_cond("m.part_role", o, v)?),
                };
                if neg {
                    format!("NOT EXISTS (SELECT 1 FROM assets m WHERE m.part_of = a.id AND {inner})")
                } else {
                    format!("a.id IN (SELECT m.part_of FROM assets m WHERE m.part_of IS NOT NULL AND {inner})")
                }
            }
            Builtin::Rating => {
                if let Val::Str(s) = v {
                    if s.eq_ignore_ascii_case("none") {
                        return Ok(if *op == Op::Ne { "a.rating > 0".into() } else { "a.rating = 0".into() });
                    }
                }
                self.num_cond("a.rating", op, v, unit_one)?
            }
            Builtin::Created => self.date_cond("a.ctime", op, v)?,
            Builtin::Modified => self.date_cond("a.mtime", op, v)?,
            Builtin::Added => self.date_cond("a.added_at", op, v)?,
            Builtin::Edited => self.date_cond("a.modified_at", op, v)?,
            Builtin::Status => {
                let code = match v.as_text().to_lowercase().as_str() {
                    "ok" | "available" | "online" => 0,
                    "offline" | "unavailable" => 1,
                    "missing" | "lost" => 2,
                    other => return Err(format!("Unknown status '{other}' (ok, offline, missing)")),
                };
                match op {
                    Op::Ne => format!("a.status <> {code}"),
                    _ => format!("a.status = {code}"),
                }
            }
            Builtin::Deleted => {
                let b = parse_bool(&v.as_text()).unwrap_or(true);
                let b = if *op == Op::Ne { !b } else { b };
                format!("a.deleted = {}", if b { 1 } else { 0 })
            }
            Builtin::Tag => self.set_cond("asset_tags", "tag_id", "tags", op, v)?,
            Builtin::Folder => self.set_cond("asset_folders", "folder_id", "folders", op, v)?,
            Builtin::FolderColor => self.folder_color(op, v)?,
            Builtin::Root => {
                let pat = format!("%{}%", escape_like(&v.as_text()));
                let p1 = self.ptext(&pat);
                let p2 = self.ptext(&pat);
                format!("a.root_id IN (SELECT id FROM roots WHERE path LIKE {p1} ESCAPE '\\' OR label LIKE {p2} ESCAPE '\\')")
            }
            Builtin::Shape => {
                let s = v.as_text().to_lowercase();
                let r = crate::core::consts::PANORAMIC_RATIO;
                let cond = match s.as_str() {
                    "landscape" => format!("(a.width > a.height AND a.width * 1.0 / a.height < {r})"),
                    "portrait" => format!("(a.height > a.width AND a.height * 1.0 / a.width < {r})"),
                    "square" => "(a.width = a.height AND a.width > 0)".to_string(),
                    "wide" | "panorama" => {
                        format!("(a.width > a.height AND a.width * 1.0 / a.height >= {r})")
                    }
                    "tall" => format!("(a.height > a.width AND a.height * 1.0 / a.width >= {r})"),
                    other => return Err(format!("Unknown shape '{other}'")),
                };
                if *op == Op::Ne {
                    format!("(NOT {cond})")
                } else {
                    cond
                }
            }
            Builtin::Color => {
                let s = v.as_text();
                if ["grayscale", "gray", "grey"].iter().any(|g| s.eq_ignore_ascii_case(g)) {
                    let c = "color_grayscale(a.palette) = 1".to_string();
                    return Ok(if *op == Op::Ne { format!("(NOT {c})") } else { c });
                }
                if matches!(op, Op::Empty | Op::NotEmpty) {
                    return Ok(format!("a.palette IS {}NULL", if *op == Op::NotEmpty { "NOT " } else { "" }));
                }
                // "#rrggbb/75" asks for an accuracy from 0 (loose) to 100 (the colour itself)
                let (hex, accuracy) = match s.split_once('/') {
                    Some((h, a)) => (h.to_string(), Some(a.trim().parse::<f64>().ok().filter(|a| (0.0..=100.0).contains(a)).ok_or(format!("'{a}' is not an accuracy (use 0 to 100)"))?)),
                    None => (s.clone(), None),
                };
                let rgb = parse_hex(&hex).ok_or(format!("'{hex}' is not a color (use #rrggbb)"))?;
                let packed = ((rgb[0] as i64) << 16) | ((rgb[1] as i64) << 8) | rgb[2] as i64;
                let limit = match accuracy {
                    Some(a) => COLOR_LOOSEST_DE2000 - a * COLOR_DE2000_PER_ACCURACY,
                    None if matches!(op, Op::Eq) => COLOR_ACCURATE_DE2000,
                    None => COLOR_SIMILAR_DE2000,
                };
                let p = self.p(Value::Integer(packed));
                let c = format!("color_similar(a.palette, {p}, {limit:?}) = 1");
                if *op == Op::Ne {
                    format!("(NOT {c})")
                } else {
                    c
                }
            }
        })
    }

    /// Field IDs of `gps_lat` and `gps_lon`, verified indexed.
    fn gps_ids(&self) -> Result<(i64, i64), String> {
        let id = |k: &str| -> Result<i64, String> {
            let f = self.cat.custom(k).ok_or("This library has no location data yet")?;
            if !f.indexed {
                return Err(crate::core::db::not_indexed(&f.key));
            }
            Ok(f.id)
        };
        Ok((id("gps_lat")?, id("gps_lon")?))
    }

    /// One value of each field at the same place in its list, as a LoRA and its strength: joined uncorrelated so SQLite drives from `field_values` indexes.
    fn with(&mut self, all: &[Expr]) -> Result<String, String> {
        let mut from = String::new();
        let mut wh = Vec::with_capacity(all.len());
        for (i, e) in all.iter().enumerate() {
            let Expr::Cmp { field, op, value } = e else { unreachable!() };
            let f = match self.cat.custom(field) {
                Some(f) if builtin_for(field).is_none() => f.clone(),
                _ => return Err(format!("WITH joins conditions on custom fields; '{field}' isn't one")),
            };
            if !f.indexed {
                return Err(crate::core::db::not_indexed(&f.key));
            }
            let w = format!("w{i}");
            if i == 0 {
                from = format!("field_values {w}");
            } else {
                from.push_str(&format!(" JOIN field_values {w} ON {w}.asset_id = w0.asset_id AND {w}.idx = w0.idx AND {w}.field_id = {}", f.id));
            }
            let cond = self.value_cond(&f, &w, op, value)?;
            if i == 0 {
                wh.push(format!("w0.field_id = {} AND {cond}", f.id));
            } else {
                wh.push(cond);
            }
        }
        Ok(format!("a.id IN (SELECT w0.asset_id FROM {from} WHERE {})", wh.join(" AND ")))
    }

    /// A condition one stored value of the field meets, in table `alias`.
    fn value_cond(&mut self, f: &FieldDef, alias: &str, op: &Op, v: &Val) -> Result<String, String> {
        let (num, text) = (format!("{alias}.num_value"), format!("{alias}.text_value"));
        let is_date = matches!(f.ftype, FieldType::Date | FieldType::DateTime);
        Ok(match op {
            Op::Empty | Op::NotEmpty | Op::NotIn | Op::HasAll | Op::HasNone => return Err("WITH takes conditions a single value can meet, such as =, >=, BETWEEN or CONTAINS".into()),
            Op::Ne if is_date => format!("NOT ({})", self.date_cond(&num, &Op::Eq, v)?),
            Op::Ne if f.ftype.is_numeric() => format!("NOT ({})", self.num_cond(&num, &Op::Eq, v, unit_one)?),
            Op::Ne => format!("NOT ({})", self.string_cond(&text, &Op::Eq, v)?),
            Op::NotContains => format!("NOT ({})", self.string_cond(&text, &Op::Contains, v)?),
            _ if is_date => self.date_cond(&num, op, v)?,
            _ if f.ftype.is_numeric() => self.num_cond(&num, op, v, unit_one)?,
            _ => self.string_cond(&text, op, v)?,
        })
    }

    fn custom(&mut self, f: &FieldDef, op: &Op, v: &Val) -> Result<String, String> {
        let fid = f.id;
        // positive filters use uncorrelated IN so SQLite drives from idx_fv_text / idx_fv_num; negations keep NOT EXISTS
        let has = move |inner: String| format!("a.id IN (SELECT fv.asset_id FROM field_values fv WHERE fv.field_id = {fid} AND {inner})");
        let not_has = move |inner: String| format!("NOT EXISTS (SELECT 1 FROM field_values fv WHERE fv.asset_id = a.id AND fv.field_id = {fid} AND {inner})");
        let numeric = f.ftype.is_numeric();
        let is_date = matches!(f.ftype, FieldType::Date | FieldType::DateTime);
        Ok(match op {
            Op::Empty => not_has("(fv.num_value IS NOT NULL OR COALESCE(fv.text_value,'') <> '')".into()),
            Op::NotEmpty => has("(fv.num_value IS NOT NULL OR COALESCE(fv.text_value,'') <> '')".into()),
            Op::Ne if f.ftype == FieldType::Boolean => {
                let b = parse_bool(&v.as_text()).map_err(|_| format!("'{}' is not true/false", v.as_text()))?;
                if !b { has("fv.num_value = 1".into()) } else { not_has("fv.num_value = 1".into()) }
            }
            Op::Ne if is_date => not_has(self.date_cond("fv.num_value", &Op::Eq, v)?),
            Op::Ne if numeric => not_has(self.num_cond("fv.num_value", &Op::Eq, v, unit_one)?),
            Op::Ne => not_has(self.string_cond("fv.text_value", &Op::Eq, v)?),
            Op::NotIn | Op::HasNone => {
                let inner = if numeric { self.num_cond("fv.num_value", &Op::In, v, unit_one)? } else { self.string_cond("fv.text_value", &Op::In, v)? };
                not_has(inner)
            }
            Op::NotContains => not_has(self.string_cond("fv.text_value", &Op::Contains, v)?),
            Op::HasAll => {
                let mut parts = Vec::new();
                for x in v.items() {
                    let inner = if numeric { self.num_cond("fv.num_value", &Op::Eq, &x, unit_one)? } else { self.string_cond("fv.text_value", &Op::Eq, &x)? };
                    parts.push(has(inner));
                }
                format!("({})", parts.join(" AND "))
            }
            _ if is_date => has(self.date_cond("fv.num_value", op, v)?),
            Op::Contains | Op::Starts | Op::Ends | Op::Matches => has(self.string_cond("fv.text_value", op, v)?),
            _ if f.ftype == FieldType::Boolean => {
                let b = parse_bool(&v.as_text()).map_err(|_| format!("'{}' is not true/false", v.as_text()))?;
                if b { has("fv.num_value = 1".into()) } else { not_has("fv.num_value = 1".into()) }
            }
            _ if numeric => has(self.num_cond("fv.num_value", op, v, unit_one)?),
            // a number kept as text (such as a seed) mirrors its parsed numeric shadow in num_value, indexed by idx_fv_num
            Op::Lt | Op::Le | Op::Gt | Op::Ge if matches!(v, Val::Num(n, None) if n.is_finite()) => {
                let Val::Num(n, _) = v else { unreachable!() };
                let sym = match op {
                    Op::Lt => "<",
                    Op::Le => "<=",
                    Op::Gt => ">",
                    _ => ">=",
                };
                let p = self.pnum(*n);
                has(format!("fv.num_value IS NOT NULL AND fv.num_value {sym} {p}"))
            }
            Op::Similar => {
                if matches!(f.ftype, FieldType::SingleSelect | FieldType::MultiSelect) {
                    has(self.string_cond("fv.text_value", &Op::Eq, v)?)
                } else {
                    has(self.string_cond("fv.text_value", &Op::Contains, v)?)
                }
            }
            _ => has(self.string_cond("fv.text_value", op, v)?),
        })
    }
}

pub fn parse_bool(s: &str) -> Result<bool, ()> {
    match s.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" | "y" => Ok(true),
        "false" | "no" | "0" | "off" | "n" => Ok(false),
        _ => Err(()),
    }
}

pub fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}
