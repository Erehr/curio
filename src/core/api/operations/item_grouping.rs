//! Grouping items: files grouped into one item and split apart again, a part's file let go, and the library's grouping rules.

use crate::core::api::operations::write;
use std::collections::HashMap;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{read, with_library, ApiError, ApiResult, Ctx};
use crate::core::library::changes::Changed;
use crate::core::model::*;
use crate::core::library::edits::{self};
use crate::core::api::operations::job_or_queued;
use crate::core::api::operations::libraries::Nothing;
use crate::core::api::operations::done;
use crate::core::api::operations::item_reads::OneItem;
use crate::core::api::operations::uids;
use crate::core::api::operations::FILE_WAIT;
use crate::core::api::operations::job_for;
use crate::core::api::operations::bad;
use crate::core::api::operations::one_id;
use crate::core::api::operations::item_reads::Items;
use crate::core::api::operations::ids_of;
use crate::core::api::operations::read_checked;
use crate::core::api::operations::items_of;
use crate::core::api::operations::ItemRef;

#[derive(Deserialize, JsonSchema)]
pub(super) struct GroupParams {
    /// The item the files join.
    id: ItemRef,
    /// The files that become part of it.
    members: Vec<ItemRef>,
    /// What they are to it, such as preview or metadata; `part` unless set.
    #[serde(default)]
    role: Option<String>,
}

pub(super) fn item_group(c: &Ctx, p: GroupParams) -> ApiResult {
    let item = one_id(c, &p.id)?;
    let members = ids_of(c, &p.members)?;
    let role = p.role.unwrap_or_else(|| "part".into());
    if !crate::core::grouping::companions::valid_role(&role) {
        return Err(bad("`role` is letters, digits, - or _"));
    }
    write(c, move |db| {
        let name: String = db.conn.query_row("SELECT name FROM assets WHERE id = ?", [item], |r| r.get(0)).map_err(|e| e.to_string())?;
        let stem = crate::core::grouping::companions::stem(&name).to_lowercase();
        for m in &members {
            let a = db.asset(*m)?.ok_or("no such item")?;
            // a file named after the item follows its name; any other keeps its own
            let suffix = (a.name.len() > stem.len() && a.name.is_char_boundary(stem.len()) && a.name[..stem.len()].to_lowercase() == stem).then(|| a.name[stem.len()..].to_string());
            db.group(item, *m, &role, "user", suffix.as_deref(), false)?;
        }
        Ok(Changed::items(&[item]).and(Changed::removed(&members, false)))
    })
}

pub(super) fn item_split(c: &Ctx, p: Items) -> ApiResult {
    let ids = items_of(c, &p.ids)?;
    let out = job_for(c, FILE_WAIT, move |db, _| {
        let mut back = vec![];
        for id in &ids {
            back.extend(db.split(*id)?);
        }
        let out = uids(db, &back)?;
        Ok((out, Changed::items(&ids).and(Changed::added(&back, "split"))))
    })?;
    Ok(json!({"ids": out}))
}

pub(super) fn item_forget_file(c: &Ctx, p: OneItem) -> ApiResult {
    let id = one_id(c, &p.id)?;
    let gone = read(c.host, |db| Ok(db.asset(id)?.map(|a| a.part_of.is_some() && a.status != crate::core::model::Availability::Ok)))?;
    if gone != Some(true) {
        return Err(bad("Only a file grouped into an item that is missing or offline can be forgotten"));
    }
    write(c, move |db| {
        let item = db.detach(id)?;
        Ok(edits::trash(db, &[id], true)?.and(Changed::items(&[item])))
    })?;
    with_library(c.host, |lib| {
        lib.purge(Some(vec![id]));
        Ok(())
    })?;
    Ok(done())
}

pub(super) fn item_allow_grouping(c: &Ctx, p: Items) -> ApiResult {
    let ids = items_of(c, &p.ids)?;
    write(c, move |db| {
        db.allow_grouping(&ids)?;
        Ok(Changed::items(&ids))
    })
}

fn grouping_rules(c: &Ctx) -> Result<Vec<crate::core::grouping::companions::Rule>, ApiError> {
    Ok(with_library(c.host, |lib| Ok(lib.ctx_scan_options().companions))?)
}

pub(super) fn library_grouping(c: &Ctx, _: Nothing) -> ApiResult {
    let rules = grouping_rules(c)?;
    read_checked(c, |db| {
        let mut st = db.conn.prepare("SELECT part_rule, count(*) FROM assets WHERE part_of IS NOT NULL AND deleted = 0 GROUP BY part_rule").map_err(|e| e.to_string())?;
        let counts: HashMap<String, i64> = st.query_map([], |r| Ok((r.get::<_, Option<String>>(0)?.unwrap_or_default(), r.get(1)?))).map_err(|e| e.to_string())?.collect::<Result<_, _>>().map_err(|e| e.to_string())?;
        let mut out: Vec<Value> = rules.iter().map(|r| json!({"id": r.id, "owner": r.id.rsplit_once('/').map(|x| x.0).unwrap_or(""), "role": r.role, "grouped": counts.get(&r.id).copied().unwrap_or(0), "exts": r.exts, "files": r.templates.iter().map(|t| t.text()).collect::<Vec<_>>()})).collect();
        // groups of rules no longer in force, and those made by hand
        let mut rest: Vec<(&String, &i64)> = counts.iter().filter(|(k, _)| !rules.iter().any(|r| &r.id == *k)).collect();
        rest.sort();
        out.extend(rest.into_iter().map(|(id, n)| json!({"id": id, "owner": id.rsplit_once('/').map(|x| x.0).unwrap_or(""), "role": id.rsplit_once('/').map(|x| x.1).unwrap_or(id), "grouped": n})));
        Ok(Ok(json!({"rules": out})))
    })
}

pub(super) fn library_grouping_survey(c: &Ctx, _: Nothing) -> ApiResult {
    let rules = grouping_rules(c)?;
    let counts = job_for(c, FILE_WAIT, move |db, ctx| Ok((crate::core::grouping::companions::survey(db, &ctx.paths, &rules)?, Changed::default())))?;
    Ok(json!({"rules": counts.into_iter().map(|(id, count)| json!({"id": id, "count": count})).collect::<Vec<_>>()}))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct GroupingApply {
    /// Rule ids, as `library.grouping` lists them; every rule unless set.
    #[serde(default)]
    rules: Option<Vec<String>>,
}

pub(super) fn library_grouping_apply(c: &Ctx, p: GroupingApply) -> ApiResult {
    let mut rules = grouping_rules(c)?;
    if let Some(only) = &p.rules {
        rules.retain(|r| only.contains(&r.id));
    }
    let n = job_or_queued(c, FILE_WAIT, move |db, ctx| {
        let note = |done: usize, of: usize| *ctx.busy.lock() = Some(format!("Grouping files {done}/{of}"));
        let pairs = crate::core::grouping::companions::group_existing(db, &ctx.paths, &rules, &note, &ctx.cancel);
        *ctx.busy.lock() = None;
        let pairs = pairs?;
        let items: Vec<AssetId> = pairs.iter().map(|p| p.0).collect();
        let members: Vec<AssetId> = pairs.iter().map(|p| p.1).collect();
        Ok((pairs.len(), Changed::items(&items).and(Changed::removed(&members, false))))
    })?;
    Ok(n.map_or_else(|| json!({"queued": true}), |n| json!({"count": n})))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct GroupingSplit {
    /// A rule id, as `library.grouping` lists it.
    rule: String,
}

pub(super) fn library_grouping_split(c: &Ctx, p: GroupingSplit) -> ApiResult {
    let n = job_or_queued(c, FILE_WAIT, move |db, _| {
        let (items, back) = db.split_rule(&p.rule)?;
        Ok((back.len(), Changed::items(&items).and(Changed::added(&back, "split"))))
    })?;
    Ok(n.map_or_else(|| json!({"queued": true}), |n| json!({"count": n})))
}

pub(super) fn item_detach(c: &Ctx, p: OneItem) -> ApiResult {
    let id = one_id(c, &p.id)?;
    write(c, move |db| {
        let item = db.detach(id)?;
        Ok(Changed::items(&[item]).and(Changed::added(&[id], "split")))
    })
}
