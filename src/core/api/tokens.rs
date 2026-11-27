//! Web tokens: made in Settings › API, kept hashed in this computer's app settings, never in a library.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Perm;

pub const PREFIX: &str = "curio_";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiToken {
    pub id: String,
    pub name: String,
    /// The secret itself is shown once, when the token is made.
    pub hash: String,
    #[serde(default)]
    pub perms: Vec<String>,
    /// Browser origins allowed to use it, such as `chrome-extension://<id>`; empty for any.
    #[serde(default)]
    pub origins: Vec<String>,
    /// Ids of the libraries it may reach; empty for all.
    #[serde(default)]
    pub libraries: Vec<String>,
    #[serde(default)]
    pub created: i64,
    /// Made by pairing a browser extension, which never reaches files outside the library nor deletes any (`pair::REFUSED`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub paired: bool,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

impl ApiToken {
    pub fn known_perms(&self) -> Vec<Perm> {
        self.perms.iter().filter_map(|p| Perm::parse(p)).collect()
    }

    pub fn has_perm(&self, p: Perm) -> bool {
        self.perms.iter().any(|s| s == p.name())
    }
}

pub fn hash(secret: &str) -> String {
    blake3::hash(secret.trim().as_bytes()).to_hex().to_string()
}

/// A new token and its secret.
pub fn make(name: &str, perms: Vec<Perm>, origins: Vec<String>, libraries: Vec<String>) -> (ApiToken, String) {
    let secret = format!("{PREFIX}{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let perms: Vec<String> = Perm::FOR_TOKENS.into_iter().filter(|p| perms.contains(p)).map(|p| p.name().to_string()).collect();
    let origins = origins.iter().map(|o| clean_origin(o)).filter(|o| !o.is_empty()).collect();
    let name = if name.trim().is_empty() { "Token".to_string() } else { name.trim().to_string() };
    let t = ApiToken { id: crate::core::model::new_uid(), name, hash: hash(&secret), perms, origins, libraries, created: crate::core::model::now_ms(), paired: false, extra: BTreeMap::new() };
    (t, secret)
}

pub fn find<'a>(tokens: &'a [ApiToken], secret: &str) -> Option<&'a ApiToken> {
    let h = hash(secret);
    tokens.iter().find(|t| t.hash == h)
}

/// The secret from `Authorization: Bearer <secret>`.
pub fn bearer(header: Option<&str>) -> Option<&str> {
    let v = header?.trim();
    let (scheme, rest) = v.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then(|| rest.trim()).filter(|s| !s.is_empty())
}

pub fn clean_origin(o: &str) -> String {
    o.trim().trim_end_matches('/').to_lowercase()
}

pub fn is_loopback_origin(o: &str) -> bool {
    let name = o.strip_prefix("http://").or_else(|| o.strip_prefix("https://")).unwrap_or("");
    let name = if name.starts_with('[') { name.split_inclusive(']').next().unwrap_or("") } else { name.split(':').next().unwrap_or("") };
    matches!(name, "127.0.0.1" | "localhost" | "[::1]")
}

/// Requests without an Origin come from programs, not web pages; pages on this computer are let through too.
pub fn origin_allowed(t: &ApiToken, origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(o) if is_loopback_origin(o) => true,
        Some(o) => t.origins.is_empty() || t.origins.contains(&clean_origin(o)),
    }
}
