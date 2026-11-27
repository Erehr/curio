//! The example plugins *Create an example plugin* writes, from the templates in `examples/`.

use std::path::{Path, PathBuf};

fn example_files(kind: &str) -> Option<&'static [(&'static str, &'static str)]> {
    Some(match kind {
        "window" => &[("plugin.json", include_str!("examples/window/plugin.json")), ("index.html", include_str!("examples/window/index.html")), ("window.js", include_str!("examples/window/window.js"))],
        "tab" => &[("plugin.json", include_str!("examples/tab/plugin.json")), ("index.html", include_str!("examples/tab/index.html")), ("tab.js", include_str!("examples/tab/tab.js"))],
        "widget" => &[("plugin.json", include_str!("examples/widget/plugin.json")), ("widget.html", include_str!("examples/widget/widget.html")), ("widget.js", include_str!("examples/widget/widget.js"))],
        "viewer" => &[("plugin.json", include_str!("examples/viewer/plugin.json")), ("viewer.html", include_str!("examples/viewer/viewer.html")), ("viewer.js", include_str!("examples/viewer/viewer.js"))],
        "program" => &[("plugin.json", include_str!("examples/program/plugin.json")), ("program.py", include_str!("examples/program/program.py"))],
        "url" => &[("plugin.json", include_str!("examples/url/plugin.json")), ("index.html", include_str!("examples/url/index.html")), ("links.js", include_str!("examples/url/links.js"))],
        _ => return None,
    })
}

/// Writes a small working plugin of one kind into `dir`, in a folder of its own; a second one of the same kind gets a numbered folder and id. Answers the plugin's folder and id.
pub fn write_example(dir: &Path, kind: &str) -> Result<(PathBuf, String), String> {
    let files = example_files(kind).ok_or_else(|| format!("No example of the kind {kind}"))?;
    let mut n = 1;
    let (folder, id) = loop {
        let suffix = if n == 1 { String::new() } else { n.to_string() };
        let folder = dir.join(format!("example-{kind}{suffix}"));
        if !folder.exists() {
            break (folder, format!("example.{kind}{suffix}"));
        }
        n += 1;
    };
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    for (name, text) in files {
        let text = if *name == "plugin.json" { text.replacen(&format!("\"example.{kind}"), &format!("\"{id}"), 1) } else { text.to_string() };
        std::fs::write(folder.join(name), text).map_err(|e| e.to_string())?;
    }
    Ok((folder, id))
}
