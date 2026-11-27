// Hide the console window on Windows release builds.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;

use curio::desktop::StartOptions;

const USAGE: &str = "curio [--library <folder>] [--welcome] [--portable] [--tray] [--background] [--headless] [--safe-mode] [file ...]
  file          a library folder (or a file inside one) opens that library; other files are added to the library that opens
  --library     open this library folder
  --welcome     start on the welcome page instead of reopening the last library
  --portable    keep settings, plugins and new libraries in a `Curio data` folder next to the program (made if missing; while it exists, later starts are portable without the flag)
  --tray        open the last library with its window kept in the tray (how starting with the system starts it)
  --background  open behind other windows without taking the focus, as do the windows it starts, and send no desktop notifications (for test runs; also CURIO_BACKGROUND=1)
  --headless    hold the --library for the web API without a window; it shows its window when the library is opened, and ends after a minute with nothing to do
  --safe-mode   ignore themes, accent overrides and user CSS";

fn main() {
    let (link, args) = curio::desktop::system::scheme::start_args(std::env::args().skip(1).collect());
    curio::core::system::app_dirs::init(args.iter().any(|a| a == "--portable"));
    let mut opts = StartOptions { link, ..Default::default() };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--library" => {
                // relative paths would break the long-path forms used for moves on Windows
                opts.library = args.get(i + 1).map(|p| std::path::absolute(p).unwrap_or_else(|_| PathBuf::from(p)));
                i += 1;
            }
            "--safe-mode" => opts.safe_mode = true,
            "--welcome" => opts.no_library = true,
            "--portable" => {}
            "--background" => std::env::set_var(curio::desktop::BACKGROUND_ENV, "1"),
            "--relaunch" => opts.relaunch = true,
            "--tray" => opts.tray = true,
            "--headless" => opts.headless = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            other if other.starts_with("--") => eprintln!("unknown argument {other}\n{USAGE}"),
            file => opts.files.push(std::path::absolute(file).unwrap_or_else(|_| PathBuf::from(file))),
        }
        i += 1;
    }
    if std::env::var("CURIO_SAFE_MODE").is_ok() {
        opts.safe_mode = true;
    }
    curio::desktop::run(opts);
}
