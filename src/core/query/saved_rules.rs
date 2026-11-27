//! Smart folder rules saved by a `.library` folder, written in the query language.

/// Translate a saved smart folder `conditions` array into our
/// query language. Limits: 30 conditions, 30 rules each (validateConditions).
/// `folder_name` names the folder a saved folder id stands for; an id it doesn't know stays as it is.
pub fn from_saved_rules(conds: &serde_json::Value, folder_name: &dyn Fn(&str) -> Option<String>) -> Result<String, String> {
    use crate::core::consts::{MAX_CONDITIONS, MAX_RULES_PER_CONDITION};
    let arr = conds.as_array().ok_or("conditions must be an array")?;
    if arr.is_empty() || arr.len() > MAX_CONDITIONS {
        return Err(format!("conditions must have 1..={MAX_CONDITIONS} entries"));
    }
    let mut groups = Vec::new();
    for c in arr {
        let rules = c["rules"].as_array().ok_or("each condition needs rules")?;
        if rules.is_empty() || rules.len() > MAX_RULES_PER_CONDITION {
            return Err(format!("each condition needs 1..={MAX_RULES_PER_CONDITION} rules"));
        }
        let join = if c["match"].as_str() == Some("OR") { " OR " } else { " AND " };
        let mut parts = Vec::new();
        for r in rules {
            parts.extend(saved_rule(r, folder_name)?);
        }
        if parts.is_empty() {
            continue;
        }
        let mut g = format!("({})", parts.join(join));
        if c["boolean"].as_str() == Some("FALSE") {
            g = format!("NOT {g}");
        }
        groups.push(g);
    }
    if groups.is_empty() {
        return Err("every saved rule is empty".into());
    }
    Ok(groups.join(" AND "))
}

fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `None` for a list rule with no values, which restricts nothing.
fn saved_rule(r: &serde_json::Value, folder_name: &dyn Fn(&str) -> Option<String>) -> Result<Option<String>, String> {
    let prop = r["property"].as_str().unwrap_or("");
    let method = r["method"].as_str().unwrap_or("");
    let val = &r["value"];
    let mut s = val.as_str().unwrap_or("").to_string();
    if prop == "shape" {
        s = match s.as_str() {
            "panoramic-landscape" => "wide".into(),
            "panoramic-portrait" => "tall".into(),
            _ => s,
        };
    }
    let nums: Vec<f64> = match val {
        serde_json::Value::Array(a) => a.iter().filter_map(|x| x.as_f64()).collect(),
        serde_json::Value::Number(n) => vec![n.as_f64().unwrap_or(0.0)],
        _ => vec![],
    };
    let mut strs: Vec<String> = match val {
        serde_json::Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(),
        _ => vec![],
    };
    // a folder rule holds the folders' ids, where queries name folders
    if prop == "folders" {
        strs = strs.into_iter().map(|id| folder_name(&id).unwrap_or(id)).collect();
    }
    let field = match prop {
        "name" => "name",
        "url" => "url",
        "annotation" | "comments" => "annotation",
        "folderName" => "folder",
        "width" => "width",
        "height" => "height",
        "fileSize" => "size",
        "duration" => "duration",
        "createTime" | "btime" => "created",
        "mtime" => "modified",
        "timestamp" => "added",
        "tags" => "tag",
        "folders" => "folder",
        "type" => "ext",
        "rating" => "rating",
        "shape" => "shape",
        "color" => "color",
        other => return Err(format!("Saved rule property '{other}' is not supported yet")),
    };
    let unit = r["unit"].as_str().unwrap_or("");
    let n0 = nums.first().copied().unwrap_or(0.0);
    let n1 = nums.get(1).copied().unwrap_or(0.0);
    let num = |n: f64| format!("{n}{unit}");
    let date = |ms: f64| {
        use chrono::TimeZone;
        chrono::Local
            .timestamp_millis_opt(ms as i64)
            .single()
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    };
    let list = |v: &[String]| v.iter().map(|x| q(x)).collect::<Vec<_>>().join(", ");
    let is_date = matches!(prop, "createTime" | "btime" | "mtime" | "timestamp");
    let list_method = matches!(method, "union" | "intersection" | "identity") || (method == "equal" && matches!(prop, "tags" | "folders")) || (method == "contain" && prop == "rating");
    if list_method && strs.is_empty() {
        return Ok(None);
    }
    Ok(Some(match method {
        "equal" if prop == "tags" || prop == "folders" => format!("{field} HAS ALL ({})", list(&strs)),
        "equal" if prop == "rating" => format!("rating = {s}"),
        "equal" => format!("{field} = {}", q(&s)),
        "unequal" => format!("{field} != {}", q(&s)),
        "startWith" => format!("{field} STARTS WITH {}", q(&s)),
        "endWith" => format!("{field} ENDS WITH {}", q(&s)),
        "contain" if prop == "rating" => format!("rating IN ({})", strs.join(", ")),
        "contain" => format!("{field} CONTAINS {}", q(&s)),
        "uncontain" => format!("{field} NOT CONTAINS {}", q(&s)),
        "empty" => format!("{field} IS EMPTY"),
        "not-empty" => format!("{field} IS NOT EMPTY"),
        "regex" => format!("{field} MATCHES {}", q(&s)),
        "=" => format!("{field} = {}", num(n0)),
        ">=" => format!("{field} >= {}", num(n0)),
        "<=" => format!("{field} <= {}", num(n0)),
        ">" => format!("{field} > {}", num(n0)),
        "<" => format!("{field} < {}", num(n0)),
        "between" if is_date => format!("{field} BETWEEN {} AND {}", date(n0), date(n1)),
        "between" => format!("{field} BETWEEN {} AND {}", num(n0), num(n1)),
        "on" => format!("{field} = {}", date(n0)),
        "before" => format!("{field} <= {}", date(n0)),
        "after" => format!("{field} >= {}", date(n0)),
        "within" => format!("{field} WITHIN {}d", n0),
        "union" => format!("{field} IN ({})", list(&strs)),
        "intersection" => format!("{field} HAS ALL ({})", list(&strs)),
        "identity" => format!("{field} HAS NONE ({})", list(&strs)),
        "similar" => format!("color ~ {s}"),
        "accuracy" => format!("color = {s}"),
        "grayscale" => "color = grayscale".into(),
        other => return Err(format!("Saved rule method '{other}' is not supported")),
    }))
}
