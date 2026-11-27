//! Long work a plugin does (a conversion, a download, a program it runs): one list per process, shown in the window's status area and told to callers as `job.*` events.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::events::Hub;
use crate::core::model::now_ms;

/// Finished jobs stay listed this long, so a page reopened later can still read the result.
const KEEP_MS: i64 = 10 * 60 * 1000;
const KEEP_FINISHED: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Done,
    Failed,
    Cancelled,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Running => "running",
            State::Done => "done",
            State::Failed => "failed",
            State::Cancelled => "cancelled",
        }
    }
}

struct Job {
    id: String,
    plugin: Option<String>,
    title: String,
    done: f64,
    total: f64,
    text: String,
    state: State,
    result: Option<Value>,
    error: Option<String>,
    started: i64,
    ended: Option<i64>,
    cancel: Arc<AtomicBool>,
    hub: Option<Arc<Hub>>,
}

impl Job {
    fn json(&self) -> Value {
        json!({"id": self.id, "plugin": self.plugin, "title": self.title, "done": self.done, "total": self.total, "text": self.text, "state": self.state.name(), "result": self.result, "error": self.error, "started": self.started, "ended": self.ended})
    }
}

static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());

fn with<T>(f: impl FnOnce(&mut Vec<Job>) -> T) -> T {
    f(&mut JOBS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn publish(hub: &Option<Arc<Hub>>, name: &str, data: Value) {
    if let Some(h) = hub {
        h.publish(name, None, data);
    }
}

/// Starts a job and answers its id; `cancel` is set when someone cancels it.
pub fn start(hub: Option<Arc<Hub>>, plugin: Option<&str>, title: &str, total: f64) -> (String, Arc<AtomicBool>) {
    let id = format!("job-{}", uuid::Uuid::new_v4().simple());
    let cancel = Arc::new(AtomicBool::new(false));
    let job = Job { id: id.clone(), plugin: plugin.map(String::from), title: title.chars().take(200).collect(), done: 0.0, total: total.max(0.0), text: String::new(), state: State::Running, result: None, error: None, started: now_ms(), ended: None, cancel: cancel.clone(), hub: hub.clone() };
    let data = job.json();
    with(|all| {
        prune(all);
        all.push(job);
    });
    publish(&hub, "job.started", data);
    (id, cancel)
}

/// Finished jobs go after ten minutes, and the oldest of them beyond fifty at once, whenever the table is touched.
fn prune(all: &mut Vec<Job>) {
    let now = now_ms();
    all.retain(|j| j.state == State::Running || j.ended.is_some_and(|e| now - e < KEEP_MS));
    let mut surplus = all.iter().filter(|j| j.state != State::Running).count().saturating_sub(KEEP_FINISHED);
    all.retain(|j| {
        let drop = surplus > 0 && j.state != State::Running;
        surplus -= drop as usize;
        !drop
    });
}

/// Why a job can't be touched by this plugin.
fn owned<'a>(all: &'a mut [Job], id: &str, plugin: Option<&str>) -> Result<&'a mut Job, String> {
    let j = all.iter_mut().find(|j| j.id == id).ok_or_else(|| format!("There is no job {id}"))?;
    if plugin.is_some() && j.plugin.as_deref() != plugin {
        return Err(format!("The job {id} belongs to another plugin"));
    }
    Ok(j)
}

/// New progress for a running job; answers whether it was cancelled, so a program reporting progress learns to stop.
pub fn progress(id: &str, plugin: Option<&str>, done: Option<f64>, total: Option<f64>, text: Option<&str>) -> Result<bool, String> {
    let (hub, data, cancelled) = with(|all| {
        let j = owned(all, id, plugin)?;
        if j.state != State::Running {
            return Ok((None, Value::Null, j.state == State::Cancelled));
        }
        if let Some(t) = total {
            j.total = t.max(0.0);
        }
        if let Some(d) = done {
            j.done = d.max(0.0);
        }
        if let Some(t) = text {
            j.text = t.chars().take(500).collect();
        }
        Ok::<_, String>((j.hub.clone(), json!({"job": j.id, "plugin": j.plugin, "done": j.done, "total": j.total, "text": j.text}), false))
    })?;
    if !data.is_null() {
        publish(&hub, "job.progress", data);
    }
    Ok(cancelled)
}

/// Lines a job's program printed, gathered by the caller into one event.
pub fn output(id: &str, pipe: &str, lines: Vec<String>) {
    let hub = with(|all| all.iter().find(|j| j.id == id).and_then(|j| j.hub.clone()).map(|h| (h, j_plugin(all, id))));
    if let Some((h, plugin)) = hub {
        h.publish("job.output", None, json!({"job": id, "plugin": plugin, "pipe": pipe, "lines": lines}));
    }
}

fn j_plugin(all: &[Job], id: &str) -> Option<String> {
    all.iter().find(|j| j.id == id).and_then(|j| j.plugin.clone())
}

/// Ends a job with its result, failed when `error` is given; a cancelled job stays cancelled.
pub fn finish(id: &str, plugin: Option<&str>, result: Option<Value>, error: Option<String>) -> Result<(), String> {
    let (hub, data) = with(|all| {
        let j = owned(all, id, plugin)?;
        if j.state != State::Running {
            // what a stopped program printed is still worth reading
            if j.state == State::Cancelled && j.result.is_none() {
                j.result = result;
            }
            return Ok((None, Value::Null));
        }
        j.ended = Some(now_ms());
        j.result = result;
        match error {
            None => {
                j.state = State::Done;
                if j.total > 0.0 {
                    j.done = j.total;
                }
            }
            Some(e) => {
                j.state = State::Failed;
                j.error = Some(e.chars().take(2000).collect());
            }
        }
        let out = (j.hub.clone(), j.json());
        prune(all);
        Ok::<_, String>(out)
    })?;
    if !data.is_null() {
        publish(&hub, "job.finished", data);
    }
    Ok(())
}

/// Cancels a running job: its program is stopped, or a page doing the work hears `job.finished` with `cancelled`.
pub fn cancel(id: &str, plugin: Option<&str>) -> Result<(), String> {
    let (hub, data) = with(|all| {
        let j = owned(all, id, plugin)?;
        if j.state != State::Running {
            return Ok((None, Value::Null));
        }
        j.cancel.store(true, Ordering::Relaxed);
        j.state = State::Cancelled;
        j.ended = Some(now_ms());
        Ok::<_, String>((j.hub.clone(), j.json()))
    })?;
    if !data.is_null() {
        publish(&hub, "job.finished", data);
    }
    Ok(())
}

/// Running jobs and those finished lately, oldest first; a plugin sees its own only.
pub fn list(plugin: Option<&str>) -> Vec<Value> {
    with(|all| {
        prune(all);
        all.iter().filter(|j| plugin.is_none() || j.plugin.as_deref() == plugin).map(Job::json).collect()
    })
}

pub fn get(id: &str, plugin: Option<&str>) -> Result<Value, String> {
    with(|all| {
        prune(all);
        owned(all, id, plugin).map(|j| j.json())
    })
}

/// The running ones, for the window's status area.
pub fn running() -> Vec<Value> {
    with(|all| all.iter().filter(|j| j.state == State::Running).map(Job::json).collect())
}
