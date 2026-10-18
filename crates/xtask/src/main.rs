//! Build tasks, run with `cargo xtask <task>` (see `.cargo/config.toml`).
//!
//! - `bundle`: builds `tally.app` in `target/release/bundle/` (macOS only).
//! - `install [--dir <folder>]`: on macOS, bundles and copies `tally.app` into
//!   `/Applications`, `~/Applications` if that isn't writable, or `<folder>`;
//!   elsewhere, runs `cargo install`.
//!   `cargo install-app` is a shortcut for this.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs};

type Result<T = ()> = std::result::Result<T, String>;

const APP_NAME: &str = "tally";
const BUNDLE_ID: &str = "dev.tally.app";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ICON_SVG: &[u8] = include_bytes!("../../tally-app/assets/icon.svg");

const USAGE: &str = "usage:
  cargo xtask bundle                    build tally.app in target/release/bundle (macOS)
  cargo xtask install [--dir <folder>]  install tally (macOS: into Applications)
  cargo install-app                     same as `cargo xtask install`";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["bundle"] => bundle().map(|app| println!("built {}", app.display())),
        ["install"] => install(None),
        ["install", "--dir", dir] => install(Some(PathBuf::from(dir))),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("xtask lives at crates/xtask")
        .to_path_buf()
}

fn target_dir(root: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
}

fn cargo() -> Command {
    Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn run(command: &mut Command) -> Result {
    let status = command
        .status()
        .map_err(|e| format!("could not run {command:?}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} failed ({status})"))
    }
}

fn install(dir: Option<PathBuf>) -> Result {
    if !cfg!(target_os = "macos") {
        let root = workspace_root();
        return run(cargo()
            .args(["install", "--locked", "--path"])
            .arg(root.join("crates/tally-app")));
    }

    let app = bundle()?;
    let dir = match dir {
        Some(dir) => dir,
        None => default_dir()?,
    };
    let installed = dir.join(format!("{APP_NAME}.app"));
    if installed.exists() {
        fs::remove_dir_all(&installed)
            .map_err(|e| format!("could not replace {}: {e}", installed.display()))?;
    }
    fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    // `ditto` keeps the bundle's signature and extended attributes intact.
    run(Command::new("ditto").arg(&app).arg(&installed))?;
    // Ask Launch Services to pick up the new icon and version now, not on next login.
    let _ = Command::new(
        "/System/Library/Frameworks/CoreServices.framework/Frameworks/\
         LaunchServices.framework/Support/lsregister",
    )
    .arg("-f")
    .arg(&installed)
    .status();
    println!("installed {}", installed.display());

    let running = Command::new("pgrep")
        .args(["-x", APP_NAME])
        .output()
        .is_ok_and(|out| out.status.success());
    if running {
        println!("tally is already running: quit it from the menu bar, then open tally again.");
    } else {
        run(Command::new("open").arg(&installed))?;
    }
    Ok(())
}

/// `/Applications` when this account can write to it, otherwise `~/Applications`
/// (standard users on managed Macs can't). Spotlight and Launchpad find both.
fn default_dir() -> Result<PathBuf> {
    let shared = PathBuf::from("/Applications");
    let probe = shared.join(".tally-install-probe");
    if fs::File::create(&probe).is_ok() {
        let _ = fs::remove_file(&probe);
        return Ok(shared);
    }
    let home = env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join("Applications"))
}

/// Builds `tally.app`: the release binary, an icon rendered from `icon.svg`, and an
/// `Info.plist` that keeps tally out of the Dock. Signed ad hoc (no Apple account).
fn bundle() -> Result<PathBuf> {
    if !cfg!(target_os = "macos") {
        return Err("bundling makes a macOS .app, so it only runs on macOS".into());
    }
    let root = workspace_root();
    run(cargo()
        .current_dir(&root)
        .args(["build", "--release", "--locked", "--package", "tally"]))?;

    let release = target_dir(&root).join("release");
    let out = release.join("bundle");
    let app = out.join(format!("{APP_NAME}.app"));
    if app.exists() {
        fs::remove_dir_all(&app).map_err(|e| format!("could not clear {}: {e}", app.display()))?;
    }
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    for dir in [&macos, &resources] {
        fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }

    fs::copy(release.join(APP_NAME), macos.join(APP_NAME))
        .map_err(|e| format!("could not copy the tally binary: {e}"))?;
    fs::write(contents.join("Info.plist"), info_plist())
        .map_err(|e| format!("could not write Info.plist: {e}"))?;
    make_icns(&out, &resources.join(format!("{APP_NAME}.icns")))?;
    run(Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(&app))?;
    Ok(app)
}

fn info_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>{APP_NAME}</string>
    <key>CFBundleDisplayName</key><string>{APP_NAME}</string>
    <key>CFBundleExecutable</key><string>{APP_NAME}</string>
    <key>CFBundleIdentifier</key><string>{BUNDLE_ID}</string>
    <key>CFBundleIconFile</key><string>{APP_NAME}</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>{VERSION}</string>
    <key>CFBundleVersion</key><string>{VERSION}</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>LSUIElement</key><true/>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#
    )
}

/// Renders the SVG at every size an `.iconset` needs, then lets `iconutil` pack them.
fn make_icns(scratch: &Path, icns: &Path) -> Result {
    let iconset = scratch.join(format!("{APP_NAME}.iconset"));
    if iconset.exists() {
        fs::remove_dir_all(&iconset).map_err(|e| format!("could not clear the iconset: {e}"))?;
    }
    fs::create_dir_all(&iconset).map_err(|e| format!("could not create the iconset: {e}"))?;
    for base in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let name = if scale == 1 {
                format!("icon_{base}x{base}.png")
            } else {
                format!("icon_{base}x{base}@2x.png")
            };
            render_png(base * scale)?
                .save_png(iconset.join(&name))
                .map_err(|e| format!("could not write {name}: {e}"))?;
        }
    }
    run(Command::new("iconutil")
        .args(["--convert", "icns", "--output"])
        .arg(icns)
        .arg(&iconset))?;
    fs::remove_dir_all(&iconset).map_err(|e| format!("could not clean up the iconset: {e}"))
}

fn render_png(size: u32) -> Result<resvg::tiny_skia::Pixmap> {
    let tree = resvg::usvg::Tree::from_data(ICON_SVG, &resvg::usvg::Options::default())
        .map_err(|e| format!("icon.svg is invalid: {e}"))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).ok_or("bad icon size")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_renders_with_a_transparent_corner_and_an_opaque_middle() {
        let pixmap = render_png(64).unwrap();
        assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 0);
        assert_eq!(pixmap.pixel(32, 32).unwrap().alpha(), 255);
    }

    #[test]
    fn plist_carries_the_workspace_version_and_hides_the_dock_icon() {
        let plist = info_plist();
        assert!(plist.contains(&format!("<string>{VERSION}</string>")));
        assert!(plist.contains("<key>LSUIElement</key><true/>"));
    }
}
