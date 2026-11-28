//! What a change touched, as the window and the API hear of it: its parts and aspects.

use serde::Serialize;
use crate::core::model::*;
use crate::core::scan::AssetEvent;

/// Lists of a library beside its items; each has its own `*.changed` event in the API.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parts(u8);

impl Parts {
    pub const FOLDERS: Parts = Parts(1);
    pub const SMART: Parts = Parts(2);
    pub const TAGS: Parts = Parts(4);
    pub const TAG_GROUPS: Parts = Parts(8);
    pub const FIELDS: Parts = Parts(16);
    pub const LOCATIONS: Parts = Parts(32);
    /// The sidebar's own lists, such as favourites, which the API doesn't announce.
    pub const SIDEBAR: Parts = Parts(64);
    pub const ALL: Parts = Parts(127);
    /// With the event each part has.
    pub const EVENTS: [(Parts, &'static str); 6] = [(Parts::FOLDERS, "folders.changed"), (Parts::SMART, "smartFolders.changed"), (Parts::TAGS, "tags.changed"), (Parts::TAG_GROUPS, "tagGroups.changed"), (Parts::FIELDS, "fields.changed"), (Parts::LOCATIONS, "locations.changed")];

    pub fn has(self, p: Parts) -> bool {
        self.0 & p.0 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Parts {
    type Output = Parts;
    fn bitor(self, o: Parts) -> Parts {
        Parts(self.0 | o.0)
    }
}

/// What about an item changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Aspect {
    /// What the user sets: name, notes, link, rating, tags, folders, fields, trash.
    #[default]
    Metadata,
    /// The file or what is read from it: content, size, place on disk, being missing.
    File,
    Thumbnail,
    Palette,
}

/// What an edit or a background job changed: the window refreshes from it, and the API announces it.
#[derive(Clone, Debug, Default)]
pub struct Changed {
    pub ids: Vec<AssetId>,
    pub aspect: Aspect,
    /// New items, and how they came (`window`, `api`, `autoImport`, `scan`, `duplicate`, `restore`).
    pub added: Vec<AssetId>,
    pub how: &'static str,
    /// Items moved to the trash, or with `purged` deleted for good.
    pub removed: Vec<AssetId>,
    pub purged: bool,
    /// The API's ids of items whose rows are gone by the time the change is told.
    pub uids: Vec<(AssetId, String)>,
    pub parts: Parts,
    /// The current result list may be different now.
    pub requery: bool,
    /// Items took a new shape with their thumbnails (a comic's or a book's cover): the list holds the same items, but the window asks for it again in a while for their shapes. Thumbnails come many a second, so this never asks at once as `requery` does.
    pub reshaped: bool,
    /// The `requestId` of the API add that made `added`.
    pub request: Option<String>,
    /// For the window alone, ahead of the change that tells of it for good: what a first scan finds as it walks, before pairing may make some of it members of others.
    pub provisional: bool,
}

impl Changed {
    /// Items' metadata; their counts in the sidebar change with it.
    pub fn items(ids: &[AssetId]) -> Self {
        Changed { ids: ids.to_vec(), requery: true, ..Default::default() }
    }
    /// Items' files, which may change where they sort.
    pub fn files(ids: &[AssetId]) -> Self {
        Changed { ids: ids.to_vec(), aspect: Aspect::File, requery: true, ..Default::default() }
    }
    /// A change that leaves the result list as it is.
    pub fn shown(ids: &[AssetId], aspect: Aspect) -> Self {
        Changed { ids: ids.to_vec(), aspect, ..Default::default() }
    }
    /// Thumbnails that gave items a new shape; without `ids`, ones already told of as made.
    pub fn reshaped(ids: &[AssetId]) -> Self {
        Changed { ids: ids.to_vec(), aspect: Aspect::Thumbnail, reshaped: true, ..Default::default() }
    }
    pub fn added(ids: &[AssetId], how: &'static str) -> Self {
        Changed { added: ids.to_vec(), how, requery: true, ..Default::default() }
    }
    pub fn removed(ids: &[AssetId], purged: bool) -> Self {
        Changed { removed: ids.to_vec(), purged, requery: true, ..Default::default() }
    }
    pub fn parts(parts: Parts) -> Self {
        Changed { parts, requery: true, ..Default::default() }
    }
    /// The sidebar's lists changed in what no search can ask for: their order, pins, groups and looks.
    pub fn looks(parts: Parts) -> Self {
        Changed { parts, ..Default::default() }
    }
    /// Anything may have changed.
    pub fn structure() -> Self {
        Changed::parts(Parts::ALL)
    }
    pub fn with(mut self, parts: Parts) -> Self {
        self.parts = self.parts | parts;
        self
    }
    /// With the API's ids of items about to be deleted.
    pub fn naming(mut self, uids: Vec<(AssetId, String)>) -> Self {
        self.uids.extend(uids);
        self
    }
    pub fn and(mut self, o: Changed) -> Self {
        self.ids.extend(o.ids);
        if o.aspect != Aspect::Metadata {
            self.aspect = o.aspect;
        }
        self.added.extend(o.added);
        if !o.how.is_empty() {
            self.how = o.how;
        }
        self.removed.extend(o.removed);
        self.purged |= o.purged;
        self.uids.extend(o.uids);
        self.parts = self.parts | o.parts;
        self.requery |= o.requery;
        self.reshaped |= o.reshaped;
        self
    }
    /// From what a scan of a location found.
    pub fn of_scan(events: &[AssetEvent]) -> Self {
        let mut c = Changed { aspect: Aspect::File, how: "scan", ..Default::default() };
        for e in events {
            match e {
                AssetEvent::Discovered(id) => c.added.push(*id),
                AssetEvent::Modified(id) | AssetEvent::Moved(id) | AssetEvent::Missing(id) => c.ids.push(*id),
            }
        }
        c.requery = !c.added.is_empty() || events.iter().any(|e| !matches!(e, AssetEvent::Modified(_)));
        c
    }
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty() && self.added.is_empty() && self.removed.is_empty() && self.parts.is_empty() && !self.requery
    }
    /// The sidebar's lists or counts may be different now.
    pub fn sidebar(&self) -> bool {
        !self.parts.is_empty() || !self.added.is_empty() || !self.removed.is_empty() || (self.requery && !self.ids.is_empty())
    }
    /// Every item it names, for the window to read again.
    pub fn all_ids(&self) -> Vec<AssetId> {
        self.ids.iter().chain(&self.added).chain(&self.removed).copied().collect()
    }
}
