//! Builds the web front end (web/ → target/web-dist/) before Tauri embeds it.
//! `npm ci` runs only when node_modules is missing, older than package.json or package-lock.json, or without vite.
//! A debug build carries the pseudo-language (`en-XA`) for checking translations: its window goes to target/web-dist-debug and is embedded from there.

use std::path::Path;
use std::process::Command;

fn npm() -> Command {
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "npm"]);
        c
    } else {
        Command::new("npm")
    }
}

fn main() {
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/icons/logo.svg");
    println!("cargo:rerun-if-changed=web/index.html");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/package-lock.json");
    println!("cargo:rerun-if-changed=web/vite.config.js");
    println!("cargo:rerun-if-changed=src/core/api/client/curio.js");
    println!("cargo:rerun-if-changed=web/locales.vite.js");
    println!("cargo:rerun-if-changed=locales");
    let pseudo = pseudo();
    let web = Path::new("web");
    if std::env::var("CURIO_SKIP_WEB").is_err() && web.join("package.json").exists() {
        install(web);
        let st = npm().args(["run", "build"]).current_dir(web).env("CURIO_PSEUDO", if pseudo { "1" } else { "0" }).status().expect("npm run build");
        assert!(st.success(), "building the web front end failed");
    }
    locales();
    version();
    tauri_build::build()
}

/// `npm ci` in `dir` when its node_modules is missing, older than its package files, or without vite.
/// `npm install --package-lock-only`, which each version bump runs, rewrites node_modules' `.package-lock.json` without installing anything, so its date alone can call a broken install current.
fn install(dir: &Path) {
    let modified = |p: std::path::PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let installed = modified(dir.join("node_modules").join(".package-lock.json"));
    let vite = dir.join("node_modules").join("vite").join("package.json").is_file();
    if installed.is_none() || !vite || modified(dir.join("package.json")) > installed || modified(dir.join("package-lock.json")) > installed {
        let st = npm().args(["ci", "--no-audit", "--no-fund"]).current_dir(dir).status().expect("npm is required to build the window (https://nodejs.org)");
        assert!(st.success(), "npm ci failed in {}", dir.display());
    }
}

/// Whether this build carries the pseudo-language: a debug build does unless `CURIO_PSEUDO=0`; a release build never.
/// It then has `cfg(curio_pseudo)`, and its window goes to target/web-dist-debug, so a release build never embeds a window built for debugging.
fn pseudo() -> bool {
    println!("cargo:rustc-check-cfg=cfg(curio_pseudo)");
    println!("cargo:rerun-if-env-changed=CURIO_PSEUDO");
    let on = std::env::var("PROFILE").as_deref() == Ok("debug") && std::env::var("CURIO_PSEUDO").as_deref() != Ok("0");
    if on {
        println!("cargo:rustc-cfg=curio_pseudo");
        println!(r#"cargo:rustc-env=TAURI_CONFIG={{"build":{{"frontendDist":"target/web-dist-debug"}}}}"#);
    }
    on
}

/// Hands the version of `tauri.conf.json` to the code as `CURIO_VERSION`.
/// `Cargo.toml`'s own version stays fixed: Cargo names build files by a hash of it, and a new one each commit left a full set of them behind.
fn version() {
    println!("cargo:rerun-if-changed=tauri.conf.json");
    let conf = std::fs::read_to_string("tauri.conf.json").expect("tauri.conf.json");
    let version = conf.split("\"version\"").nth(1).and_then(|rest| rest.split('"').nth(1)).expect("tauri.conf.json has a version");
    println!("cargo:rustc-env=CURIO_VERSION={version}");
}

/// Lists each language's `locales/<code>/native.json` for `locale::CATALOGS`, English first, so a language added there needs no code.
fn locales() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let mut codes: Vec<String> = std::fs::read_dir(&root).expect("locales").flatten().filter(|e| e.path().join("native.json").is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    codes.sort_by_key(|c| (c != "en", c.clone()));
    let mut out = String::from("&[\n");
    for c in codes {
        println!("cargo:rerun-if-changed=locales/{c}/native.json");
        out.push_str(&format!("    ({c:?}, include_str!({:?})),\n", root.join(&c).join("native.json").to_string_lossy()));
    }
    out.push(']');
    std::fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("locales.rs"), out).unwrap();
}
