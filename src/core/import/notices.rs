//! What Curio tells of an add in a system notification: the app tells of the items every add made, and the core works out from an add's pieces what failed or was there already.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Added,
    /// Skipped: the library already holds it.
    Already,
    Failed,
}

#[derive(Clone, Debug)]
pub struct Notice {
    pub kind: Kind,
    pub library: PathBuf,
    /// The items added, by id.
    pub items: Vec<String>,
    pub names: Vec<String>,
    /// The folder they went to, by name.
    pub folder: Option<String>,
    /// What failed, in words, without the address.
    pub errors: Vec<String>,
    /// The item the library already held.
    pub of: Option<String>,
    /// Downloads that may be sent again: those that failed, or were skipped as already there.
    pub again: Vec<String>,
}

impl Notice {
    /// Takes in a later add to the same library, so a run of saves is told once.
    pub fn absorb(&mut self, o: Notice) {
        self.items.extend(o.items);
        self.names.extend(o.names);
        self.errors.extend(o.errors);
        if self.folder != o.folder {
            self.folder = None;
        }
    }
}

type Sink =Box<dyn Fn(Notice) + Send + Sync>;

static SINK: OnceLock<Sink> = OnceLock::new();

/// Where notices go; the app sets it once. Without one, as in tools, they go nowhere.
pub fn set_sink(f: Sink) {
    let _ = SINK.set(f);
}

pub fn post(n: Notice) {
    if let Some(s) = SINK.get() {
        s(n);
    }
}

/// An error without the address it starts with, which a person has just sent.
fn plain(e: &str) -> String {
    match e.split_once(": ") {
        Some((head, rest)) if head.contains("://") => rest.to_string(),
        _ => e.to_string(),
    }
}

/// What is left to tell of an add made of `pieces` (`{ ids, names, errors, duplicates, folder }`, each with the download it came from, if any), its items being told of as the library adds them: what failed, or that all of it was there already.
pub fn of_rest(library: PathBuf, pieces: &[(Value, Option<String>)]) -> Option<Notice> {
    let mut n = Notice { kind: Kind::Failed, library, items: vec![], names: vec![], folder: None, errors: vec![], of: None, again: vec![] };
    let strings = |v: &Value| -> Vec<String> { v.as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect() };
    let (mut skipped, mut failed) = (vec![], vec![]);
    let mut added = false;
    for (p, record) in pieces {
        added |= p["ids"].as_array().is_some_and(|a| !a.is_empty());
        let errors = strings(&p["errors"]);
        if !errors.is_empty() && p["ids"].as_array().is_none_or(|a| a.is_empty()) {
            failed.extend(record.clone());
        }
        n.errors.extend(errors.iter().map(|e| plain(e)));
        for d in p["duplicates"].as_array().into_iter().flatten().filter(|d| d["skipped"].as_bool() == Some(true)) {
            n.of = n.of.or_else(|| d["of"].as_str().map(String::from));
            skipped.extend(record.clone());
        }
    }
    if !n.errors.is_empty() {
        n.again = failed;
        return Some(n);
    }
    if n.of.is_some() && !added {
        n.kind = Kind::Already;
        n.again = skipped;
        return Some(n);
    }
    None
}
