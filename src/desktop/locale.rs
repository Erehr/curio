//! The language Curio shows, worked out here for every window and the tray alike: the one chosen in Settings › General, else the first of the system's that Curio has.
//! Also the text Rust shows by itself (tray, notifications, dialogs), from `locales/<code>/native.json`: plain text with named placeholders.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use serde_json::Value;

/// Each language's `native.json` by code, English first, as `build.rs` found them in `locales/`.
const CATALOGS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/locales.rs"));
/// English made longer and accented, in debug builds (`web/src/shared/pseudoLanguage.js` makes the window's).
#[cfg(curio_pseudo)]
const PSEUDO: &str = "en-XA";

/// The languages this build carries.
pub fn available() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut v: Vec<&'static str> = CATALOGS.iter().map(|(c, _)| *c).collect();
    #[cfg(curio_pseudo)]
    v.push(PSEUDO);
    v
}

/// `en_US.UTF-8` and `pt_BR@euro` as `en-US` and `pt-BR`.
fn normalize(tag: &str) -> String {
    tag.split(['.', '@']).next().unwrap_or("").replace('_', "-")
}

/// The first of `wanted` that is in `have`, by its whole tag and then by its language alone (`pt-BR`, then `pt`); English when none is.
pub fn resolve<'a>(wanted: impl IntoIterator<Item = String>, have: &[&'a str]) -> &'a str {
    for w in wanted {
        let w = normalize(&w);
        if let Some(c) = have.iter().find(|c| c.eq_ignore_ascii_case(&w)) {
            return c;
        }
        let base = w.split('-').next().unwrap_or("");
        if let Some(c) = have.iter().find(|c| !base.is_empty() && c.split('-').next().is_some_and(|b| b.eq_ignore_ascii_case(base))) {
            return c;
        }
    }
    "en"
}

/// The languages the system's user prefers, most preferred first.
#[cfg(windows)]
fn system_languages() -> Vec<String> {
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    let (mut count, mut len) = (0u32, 0u32);
    if unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut len) }.is_err() || len == 0 {
        return vec![];
    }
    let mut buf = vec![0u16; len as usize];
    if unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, Some(windows::core::PWSTR(buf.as_mut_ptr())), &mut len) }.is_err() {
        return vec![];
    }
    // names separated by a nul, the list ended by two
    String::from_utf16_lossy(&buf).split('\0').filter(|s| !s.is_empty()).map(String::from).collect()
}

/// The languages the system's user prefers, most preferred first: `LANGUAGE`'s list, then the locale in force.
#[cfg(not(windows))]
fn system_languages() -> Vec<String> {
    let mut out: Vec<String> = std::env::var("LANGUAGE").unwrap_or_default().split(':').filter(|s| !s.is_empty()).map(String::from).collect();
    out.extend(["LC_ALL", "LC_MESSAGES", "LANG"].iter().filter_map(|k| std::env::var(k).ok()).find(|v| !v.is_empty() && v != "C" && v != "POSIX"));
    out
}

/// The system's language among those this build carries.
pub fn system() -> &'static str {
    static SYSTEM: OnceLock<&'static str> = OnceLock::new();
    SYSTEM.get_or_init(|| resolve(system_languages(), &available()))
}

/// The language chosen in Settings › General, read again only when the settings file has changed.
fn chosen() -> Option<String> {
    static SEEN: Mutex<Option<(Option<SystemTime>, Option<String>)>> = Mutex::new(None);
    let path = crate::core::system::app_dirs::config_dir().join("ui-settings.json");
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, language)) = seen.as_ref() {
        if *at == modified {
            return language.clone();
        }
    }
    let ui: Value = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let language = ui["language"].as_str().filter(|s| !s.is_empty()).map(String::from);
    *seen = Some((modified, language.clone()));
    language
}

/// The language shown now: the chosen one when this build carries it, else the system's.
pub fn current() -> String {
    // a language to check a build in without changing the settings
    #[cfg(curio_pseudo)]
    if let Some(l) = std::env::var("CURIO_LANGUAGE").ok().filter(|l| available().contains(&l.as_str())) {
        return l;
    }
    match chosen() {
        Some(c) if available().contains(&c.as_str()) => c,
        _ => system().to_string(),
    }
}

fn catalogs() -> &'static HashMap<&'static str, Value> {
    static PARSED: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    PARSED.get_or_init(|| CATALOGS.iter().map(|(c, text)| (*c, serde_json::from_str(text).unwrap_or_default())).collect())
}

fn lookup(code: &str, key: &str) -> Option<&'static str> {
    key.split('.').try_fold(catalogs().get(code)?, |v, k| v.get(k))?.as_str()
}

/// Each `{name}` of `message` replaced by its value, in one pass: what goes in is never looked at again, so a library called `Draft {count}` keeps its name.
pub fn fill(message: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}').map(|close| (&after[..close], &after[close + 1..])) {
            Some((name, tail)) if args.iter().any(|(n, _)| *n == name) => {
                out.push_str(args.iter().find(|(n, _)| *n == name).map(|(_, v)| *v).unwrap_or(""));
                rest = tail;
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The native text of `key` in the language shown now, English when that language lacks it, with its `{name}`s filled in.
pub fn text(key: &str, args: &[(&str, &str)]) -> String {
    text_in(&current(), key, args)
}

/// The native text of `key` in the language `code`, English when that language lacks it.
pub fn text_in(code: &str, key: &str, args: &[(&str, &str)]) -> String {
    #[cfg(curio_pseudo)]
    if code == PSEUDO {
        return fill(&pseudo(lookup("en", key).unwrap_or(key)), args);
    }
    fill(lookup(code, key).or_else(|| lookup("en", key)).unwrap_or(key), args)
}

/// English as the pseudo-language has it: accented, a third longer, in brackets; placeholders stay.
#[cfg(curio_pseudo)]
fn pseudo(s: &str) -> String {
    const PLAIN: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const ACCENTED: &str = "àƀçđéƒĝĥîĵķĺɱñöþǫŕšţûṽŵẋýžÀƁÇĐÉƑĜĤÎĴĶĹṀÑÖÞǪŔŠŢÛṼŴẊÝŽ";
    let mut out = String::from("[");
    let mut letters = 0usize;
    let mut inside = false;
    for c in s.chars() {
        match c {
            '{' => inside = true,
            '}' => inside = false,
            _ => {}
        }
        match PLAIN.find(c).filter(|_| !inside) {
            Some(i) => {
                letters += 1;
                out.extend(ACCENTED.chars().nth(i));
            }
            None => out.push(c),
        }
    }
    if letters > 0 {
        out.push(' ');
        out.extend(std::iter::repeat_n('·', letters.div_ceil(3)));
    }
    out.push(']');
    out
}
