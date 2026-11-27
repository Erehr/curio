//! Jobs: started, followed, finished and cancelled through the API.

use std::path::{Path, PathBuf};
use std::time::Duration;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::core::api::{ApiError, ApiResult, CallerKind, Ctx, Perm};
use crate::core::api::operations::bad;
use crate::core::api::operations::libraries::Nothing;
use crate::core::api::operations::done;
use crate::core::api::operations::not_found;
use crate::core::api::operations::plugin_data::plugin_id;

#[derive(Deserialize, JsonSchema)]
pub(super) struct JobStart {
    /// Shown in Curio's status area.
    title: String,
    /// How much there is to do, in any unit; 0 or none when it isn't known.
    #[serde(default)]
    total: Option<f64>,
}

pub(super) fn job_start(c: &Ctx, p: JobStart) -> ApiResult {
    let (id, _) = crate::core::api::jobs::start(c.host.events(), c.caller.plugin.as_deref(), &p.title, p.total.unwrap_or(0.0));
    Ok(json!({ "job": id }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct JobUpdate {
    job: String,
    #[serde(default)]
    done: Option<f64>,
    #[serde(default)]
    total: Option<f64>,
    /// A line on what it is doing now.
    #[serde(default)]
    text: Option<String>,
}

/// A plugin reaches its own jobs only; the window and scripts reach every one.
fn job_owner(c: &Ctx) -> Option<String> {
    c.caller.plugin.clone()
}

pub(super) fn job_update(c: &Ctx, p: JobUpdate) -> ApiResult {
    let cancelled = crate::core::api::jobs::progress(&p.job, job_owner(c).as_deref(), p.done, p.total, p.text.as_deref()).map_err(not_found)?;
    Ok(json!({ "cancelled": cancelled }))
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct JobFinish {
    job: String,
    /// What the job made, kept for job.get for ten minutes.
    #[serde(default)]
    result: Option<Value>,
    /// What went wrong; the job is then failed.
    #[serde(default)]
    error: Option<String>,
}

pub(super) fn job_finish(c: &Ctx, p: JobFinish) -> ApiResult {
    crate::core::api::jobs::finish(&p.job, job_owner(c).as_deref(), p.result, p.error).map_err(not_found)?;
    Ok(done())
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct JobId {
    job: String,
}

pub(super) fn job_cancel(c: &Ctx, p: JobId) -> ApiResult {
    crate::core::api::jobs::cancel(&p.job, job_owner(c).as_deref()).map_err(not_found)?;
    Ok(done())
}

pub(super) fn job_list(c: &Ctx, _: Nothing) -> ApiResult {
    Ok(Value::Array(crate::core::api::jobs::list(job_owner(c).as_deref())))
}

pub(super) fn job_get(c: &Ctx, p: JobId) -> ApiResult {
    crate::core::api::jobs::get(&p.job, job_owner(c).as_deref()).map_err(not_found)
}

#[derive(Deserialize, JsonSchema)]
pub(super) struct ProcessRun {
    /// A program and its arguments; a first element naming a file in the plugin's folder runs that file (`["bin/convert", "--fast"]`), anything else is looked for on the system's path.
    #[serde(default)]
    program: Option<Vec<String>>,
    /// A tool Curio finds, which the plugin lists in its `tools`: ffmpeg, ffprobe, yt-dlp, gallery-dl, exiftool, magick or 7z.
    #[serde(default)]
    tool: Option<String>,
    /// The tool's arguments.
    #[serde(default)]
    args: Vec<String>,
    /// Text given to the program on its input.
    #[serde(default)]
    stdin: Option<String>,
    /// Shown in Curio's status area; the program's name without it.
    #[serde(default)]
    title: Option<String>,
    /// Answer when the program ends, waiting up to two minutes, with its exit code and output.
    #[serde(default)]
    wait: bool,
    /// Seconds before the program is stopped: an hour unless set, at most a day.
    #[serde(default)]
    timeout: Option<u64>,
}

/// What a run keeps of each pipe for its result.
const RUN_KEEP: usize = 1 << 20;

const RUN_WAIT: Duration = Duration::from_secs(120);

/// Where runs work, one folder each; folders a day old go when the next run starts.
fn runs_dir() -> PathBuf {
    std::env::temp_dir().join("curio-runs")
}

/// Paths a caller may name with `files.write` alone are in the folders of its own plugin's runs, while the run is listed; any other path needs `files.outside`.
pub(super) fn may_name(c: &Ctx, paths: &[String]) -> Result<(), ApiError> {
    if c.caller.may(Perm::FilesOutside) {
        return Ok(());
    }
    let runs = std::fs::canonicalize(runs_dir()).ok();
    let own = |p: &String| -> bool {
        let (Some(plugin), Some(runs), Ok(p)) = (c.caller.plugin.as_deref(), runs.as_ref(), std::fs::canonicalize(p)) else { return false };
        let Ok(rel) = p.strip_prefix(runs) else { return false };
        rel.components().next().and_then(|j| j.as_os_str().to_str()).is_some_and(|job| crate::core::api::jobs::get(job, Some(plugin)).is_ok())
    };
    if paths.iter().all(own) {
        return Ok(());
    }
    Err(ApiError::new("not_allowed", format!("Files outside the library need the files.outside permission, which {} doesn't have", c.caller.name)))
}

fn clear_old_runs() {
    let Ok(rd) = std::fs::read_dir(runs_dir()) else { return };
    let day = std::time::SystemTime::now() - Duration::from_secs(24 * 3600);
    for e in rd.flatten() {
        if e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < day) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

pub(super) fn process_run(c: &Ctx, p: ProcessRun) -> ApiResult {
    let id = plugin_id(c)?;
    let mut found = None;
    let _ = c.host.with_library(&mut |lib| found = lib.plugins().into_iter().find(|x| x.manifest.id == id));
    let plugin = found.ok_or_else(|| ApiError::new("not_allowed", format!("The plugin {id} isn't loaded")))?;
    let (argv, name) = match (p.program, p.tool) {
        (Some(prog), None) if !prog.is_empty() => {
            let name = Path::new(&prog[0]).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            (plugin.resolve(&prog), name)
        }
        (None, Some(t)) => {
            let tool = crate::core::plugins::tools::tool(&t).ok_or_else(|| bad(format!("Curio doesn't know a tool called {t}")))?;
            if !plugin.manifest.tools.iter().any(|x| x == tool.name) {
                return Err(ApiError::new("not_allowed", format!("{} doesn't list {} in its tools", plugin.manifest.name, tool.name)));
            }
            let path = crate::core::plugins::tools::find(tool.name).ok_or_else(|| ApiError::new("not_found", format!("{} isn't on this computer; Settings › Plugins › Tools says where to get it", tool.title)))?;
            let mut v = vec![path.to_string_lossy().to_string()];
            v.extend(p.args.iter().cloned());
            (v, tool.title.to_string())
        }
        _ => return Err(bad("Give either `program` or `tool`")),
    };
    clear_old_runs();
    let title = p.title.filter(|t| !t.trim().is_empty()).unwrap_or(name);
    let (job, stop) = crate::core::api::jobs::start(c.host.events(), Some(&id), &title, 0.0);
    let folder = runs_dir().join(&job);
    if let Err(e) = std::fs::create_dir_all(&folder) {
        let _ = crate::core::api::jobs::finish(&job, None, None, Some(e.to_string()));
        return Err(ApiError::new("failed", format!("Couldn't make the run's folder: {e}")));
    }
    let timeout = Duration::from_secs(p.timeout.unwrap_or(3600).clamp(1, 24 * 3600));
    let (tx, rx) = crossbeam_channel::bounded::<Value>(1);
    let (job2, folder2, plugin_id) = (job.clone(), folder.clone(), id.clone());
    let stdin = p.stdin;
    std::thread::spawn(move || {
        let token = crate::core::api::programs::issue(&plugin_id);
        let mut env = token.env.clone();
        env.push(("CURIO_JOB".into(), job2.clone()));
        env.push(("CURIO_RUN_FOLDER".into(), folder2.to_string_lossy().to_string()));
        let mut held: [(Vec<String>, std::time::Instant); 2] = [(Vec::new(), std::time::Instant::now()), (Vec::new(), std::time::Instant::now())];
        let flush = |k: usize, lines: Vec<String>| crate::core::api::jobs::output(&job2, if k == 0 { "stdout" } else { "stderr" }, lines);
        let mut each = |pipe: crate::core::system::process::Pipe, line: &str| {
            let k = if pipe == crate::core::system::process::Pipe::Out { 0 } else { 1 };
            held[k].0.push(line.chars().take(4000).collect());
            if held[k].0.len() >= 50 || held[k].1.elapsed() >= Duration::from_millis(200) {
                flush(k, std::mem::take(&mut held[k].0));
                held[k].1 = std::time::Instant::now();
            }
        };
        let r = crate::core::system::process::run_lines(&argv[0], &argv[1..], stdin.as_deref().map(str::as_bytes), Some(&folder2), &env, timeout, &stop, &mut each, RUN_KEEP);
        for (k, (lines, _)) in held.into_iter().enumerate() {
            if !lines.is_empty() {
                flush(k, lines);
            }
        }
        drop(token);
        let folder_s = folder2.to_string_lossy().to_string();
        let (result, error) = match r {
            Ok(out) => {
                let error = match out.status {
                    _ if out.stopped => None,
                    Some(0) => None,
                    code => Some(format!("{} exited with {}: {}", argv[0], code.map(|c| c.to_string()).unwrap_or_else(|| "a signal".into()), out.stderr.lines().last().unwrap_or("").trim())),
                };
                (Some(json!({"code": out.status, "stdout": out.stdout, "stderr": out.stderr, "folder": folder_s})), error)
            }
            Err(e) => (Some(json!({"folder": folder_s})), Some(e)),
        };
        let _ = crate::core::api::jobs::finish(&job2, None, result, error);
        let _ = tx.send(crate::core::api::jobs::get(&job2, None).unwrap_or_default());
    });
    let folder = folder.to_string_lossy().to_string();
    if !p.wait {
        return Ok(json!({"job": job, "folder": folder, "state": "running"}));
    }
    match rx.recv_timeout(RUN_WAIT) {
        Ok(j) => Ok(json!({"job": job, "folder": folder, "state": j["state"], "code": j["result"]["code"], "stdout": j["result"]["stdout"], "stderr": j["result"]["stderr"], "error": j["error"]})),
        Err(_) => Ok(json!({"job": job, "folder": folder, "state": "running"})),
    }
}

pub(super) fn tool_list(c: &Ctx, _: Nothing) -> ApiResult {
    // where programs are says something about the computer, so only callers that may run them learn it
    let paths = c.caller.kind == CallerKind::Window || c.caller.may(Perm::Exec);
    Ok(Value::Array(crate::core::plugins::tools::TOOLS.iter().map(|t| crate::core::plugins::tools::describe(t, paths)).collect()))
}
