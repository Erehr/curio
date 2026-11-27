//! The fields a query can name: the built-in ones and the library's own.

use std::collections::HashMap;
use crate::core::model::FieldDef;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Builtin {
    Name,
    FileName,
    Ext,
    Kind,
    Mime,
    Size,
    Width,
    Height,
    Aspect,
    Duration,
    Created,
    Modified,
    Added,
    Edited,
    Rating,
    Favorite,
    Explicit,
    Animated,
    TagCount,
    Annotation,
    Url,
    Path,
    Hash,
    Status,
    Tag,
    Folder,
    FolderColor,
    Color,
    Shape,
    Deleted,
    Root,
    Id,
    Parts,
    Part,
}

pub fn builtin_for(name: &str) -> Option<Builtin> {
    Some(match name.to_lowercase().as_str() {
        "name" | "display" | "display_name" => Builtin::Name,
        "filename" | "file" | "file_name" => Builtin::FileName,
        "ext" | "extension" | "type" | "format" => Builtin::Ext,
        "kind" | "media" => Builtin::Kind,
        "mime" => Builtin::Mime,
        "size" | "filesize" | "file_size" => Builtin::Size,
        "width" => Builtin::Width,
        "height" => Builtin::Height,
        "aspect" | "ratio" | "aspect_ratio" => Builtin::Aspect,
        "duration" | "length" => Builtin::Duration,
        "created" | "btime" | "ctime" | "date_created" => Builtin::Created,
        "modified" | "mtime" | "date_modified" => Builtin::Modified,
        "added" | "imported" | "indexed" | "date_added" | "date_imported" => Builtin::Added,
        "edited" | "updated" => Builtin::Edited,
        "rating" | "stars" | "star" => Builtin::Rating,
        "favorite" | "favourite" | "fav" => Builtin::Favorite,
        "explicit" | "nsfw" => Builtin::Explicit,
        "animated" | "animation" => Builtin::Animated,
        "tag_count" | "tagcount" => Builtin::TagCount,
        "annotation" | "note" | "notes" | "description" | "comment" => Builtin::Annotation,
        "url" | "source" | "source_url" | "link" => Builtin::Url,
        "path" | "folder_path" => Builtin::Path,
        "hash" | "content_hash" => Builtin::Hash,
        "status" | "availability" => Builtin::Status,
        "tag" | "tags" => Builtin::Tag,
        "folder" | "folders" | "collection" | "collections" => Builtin::Folder,
        "folder_color" | "folder_colour" => Builtin::FolderColor,
        "color" | "colour" => Builtin::Color,
        "shape" | "orientation" => Builtin::Shape,
        "deleted" | "trash" | "trashed" => Builtin::Deleted,
        "root" | "location" => Builtin::Root,
        "id" | "uid" => Builtin::Id,
        "parts" | "part_count" => Builtin::Parts,
        "part" => Builtin::Part,
        _ => return None,
    })
}

#[derive(Clone, Default)]
pub struct FieldCatalog {
    pub by_key: HashMap<String, FieldDef>,
}

impl FieldCatalog {
    pub fn new(fields: &[FieldDef]) -> Self {
        let mut by_key = HashMap::new();
        for f in fields {
            by_key.insert(f.key.to_lowercase(), f.clone());
        }
        for f in fields {
            let label_key = f.label.to_lowercase().replace([' ', '-'], "_");
            by_key.entry(label_key).or_insert_with(|| f.clone());
        }
        FieldCatalog { by_key }
    }
    pub fn custom(&self, name: &str) -> Option<&FieldDef> {
        self.by_key.get(&strip_field_prefix(name).to_lowercase())
    }
    /// Built-in names win; a plugin field called "size" is still reachable as `field.size`.
    pub fn is_field(&self, name: &str) -> bool {
        builtin_for(name).is_some() || self.custom(name).is_some()
    }
}

fn strip_field_prefix(name: &str) -> &str {
    name.strip_prefix("field.").unwrap_or(name)
}
