//! Tags and tag groups.

use crate::core::api::operations::write;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::core::api::{read, ApiResult, Ctx};
use crate::core::library::{changes::Changed, changes::Parts};
use crate::core::library::edits::{self, TagArgs};
use crate::core::api::operations::folders::ById;
use crate::core::api::operations::write_for;
use crate::core::api::operations::folders::given_name;
use crate::core::api::operations::item_edits::tag_ids;
use crate::core::api::operations::item_edits::one_tag;
use crate::core::api::operations::item_edits::TagRef;
use crate::core::api::operations::to_json;
use crate::core::api::operations::libraries::Nothing;

pub(super) fn tag_list(c: &Ctx, _: Nothing) -> ApiResult {
    to_json(read(c.host, |db| db.tags())?)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct OneTag {
    /// The tag's name, or its number.
    tag: TagRef,
}

pub(super) fn tag_get(c: &Ctx, p: OneTag) -> ApiResult {
    let id = one_tag(c, &p.tag)?;
    to_json(read(c.host, |db| db.tags())?.into_iter().find(|t| t.id == id))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct TagRename {
    /// The tag's name, or its number.
    tag: TagRef,
    new_name: String,
}

pub(super) fn tag_rename(c: &Ctx, p: TagRename) -> ApiResult {
    let id = one_tag(c, &p.tag)?;
    write(c, move |db| edits::tag_op(db, "rename", TagArgs { id: Some(id), name: Some(p.new_name), ..Default::default() }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct TagMerge {
    /// Tags to merge into `target`.
    sources: Vec<TagRef>,
    /// The tag they become; made when it doesn't exist.
    target: String,
}

pub(super) fn tag_merge(c: &Ctx, p: TagMerge) -> ApiResult {
    let ids = tag_ids(c, &p.sources)?;
    write(c, move |db| {
        for id in ids {
            edits::tag_op(db, "rename", TagArgs { id: Some(id), name: Some(p.target.clone()), ..Default::default() })?;
        }
        Ok(Changed::parts(Parts::TAGS | Parts::TAG_GROUPS))
    })
}

pub(super) fn tag_delete(c: &Ctx, p: OneTag) -> ApiResult {
    let id = one_tag(c, &p.tag)?;
    write(c, move |db| edits::tag_op(db, "delete", TagArgs { id: Some(id), ..Default::default() }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct TagColor {
    /// The tag's name, or its number.
    tag: TagRef,
    /// A CSS colour; null or empty clears it.
    #[serde(default)]
    color: Option<String>,
}

pub(super) fn tag_set_color(c: &Ctx, p: TagColor) -> ApiResult {
    let id = one_tag(c, &p.tag)?;
    write(c, move |db| edits::tag_op(db, "color", TagArgs { id: Some(id), color: p.color.filter(|s| !s.is_empty()), ..Default::default() }))
}

pub(super) fn tag_group_list(c: &Ctx, _: Nothing) -> ApiResult {
    to_json(read(c.host, |db| db.tag_groups())?)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct GroupCreate {
    name: String,
    #[serde(default)]
    color: Option<String>,
}

pub(super) fn tag_group_create(c: &Ctx, p: GroupCreate) -> ApiResult {
    let name = given_name(&p.name)?;
    let id = write_for(c, move |db| Ok((db.create_tag_group(&name, p.color.as_deref())?, Changed::looks(Parts::TAG_GROUPS))))?;
    Ok(json!({"success": true, "id": id}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct GroupUpdate {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    /// A CSS colour; empty clears it.
    #[serde(default)]
    color: Option<String>,
    /// A Tabler icon name; empty clears it.
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

pub(super) fn tag_group_update(c: &Ctx, mut p: GroupUpdate) -> ApiResult {
    p.name = p.name.as_deref().map(given_name).transpose()?;
    let id = Some(p.id);
    write(c, move |db| {
        let cur = db.tag_groups()?.into_iter().find(|g| Some(g.id) == id).ok_or("There is no such tag group")?;
        if p.name.is_some() {
            edits::tag_op(db, "rename_group", TagArgs { id, name: p.name, ..Default::default() })?;
        }
        if p.color.is_some() || p.icon.is_some() {
            let keep = |new: Option<String>, old: Option<String>| match new {
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s),
                None => old,
            };
            edits::tag_op(db, "group_look", TagArgs { id, name: keep(p.icon, cur.icon), color: keep(p.color, cur.color), ..Default::default() })?;
        }
        if p.description.is_some() {
            edits::tag_op(db, "group_description", TagArgs { id, name: p.description, ..Default::default() })?;
        }
        Ok(Changed::looks(Parts::TAG_GROUPS))
    })
}

pub(super) fn tag_group_delete(c: &Ctx, p: ById) -> ApiResult {
    write(c, move |db| edits::tag_op(db, "delete_group", TagArgs { id: Some(p.id), ..Default::default() }))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct GroupTags {
    group_id: i64,
    /// Tags by name or number; new names are created.
    tags: Vec<TagRef>,
}

pub(super) fn tag_group_add_tags(c: &Ctx, p: GroupTags) -> ApiResult {
    let known: Vec<TagRef> = p.tags.iter().filter(|t| matches!(t, TagRef::Id(_))).cloned().collect();
    tag_ids(c, &known)?;
    write(c, move |db| {
        for t in &p.tags {
            let id = match t {
                TagRef::Name(n) => db.get_or_create_tag(n)?,
                TagRef::Id(i) => *i,
            };
            edits::tag_op(db, "add_to_group", TagArgs { id: Some(id), group: Some(p.group_id), ..Default::default() })?;
        }
        Ok(Changed::looks(Parts::TAGS | Parts::TAG_GROUPS))
    })
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct GroupUntag {
    group_id: i64,
    /// Tags by name or number.
    tags: Vec<TagRef>,
}

pub(super) fn tag_group_remove_tags(c: &Ctx, p: GroupUntag) -> ApiResult {
    let ids = tag_ids(c, &p.tags)?;
    write(c, move |db| {
        for id in ids {
            edits::tag_op(db, "remove_from_group", TagArgs { id: Some(id), group: Some(p.group_id), ..Default::default() })?;
        }
        Ok(Changed::looks(Parts::TAGS | Parts::TAG_GROUPS))
    })
}
