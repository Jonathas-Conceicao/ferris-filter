//! Debug-only screenshot mode for `ff`: `ff --screenshot <png> [--tab t] [--popup p] [--status s]
//! [--filter f]` renders the shell once, saves the frame as a PNG and exits.
//!
//! The whole module is compiled out of release builds. The mode skips the single-instance guard,
//! the hotkey and the tray, never applies, reloads or syncs, and refuses to run unless
//! `FERRISFILTER_HOME`, `USERPROFILE` and the game folder derived from it all live under the
//! system temp directory -- so it can never read or write the real game folder or the user's live
//! `styles.json` / `tiers.json` / `prices.json`.

use std::path::{Path, PathBuf};

use super::shell::ShellTab;

/// The parsed command line of the screenshot mode.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShotArgs {
    /// Where the PNG goes.
    pub png: PathBuf,
    /// The tab to show.
    pub tab: ShellTab,
    /// A popup to open (rendered by later plans; stored here).
    #[allow(dead_code)]
    pub popup: Option<String>,
    /// A status line text to show in place of the computed one (to see truncation).
    pub status: Option<String>,
    /// A filter file copied into the temp game folder as `dynamic.filter` before the app starts.
    pub filter: Option<PathBuf>,
}

/// Parses the arguments after the program name. `Ok(None)` when `--screenshot` is absent (normal
/// launch); an unknown flag or a missing value is an `Err`.
pub(crate) fn parse(args: impl Iterator<Item = String>) -> Result<Option<ShotArgs>, String> {
    let mut png = None;
    let mut tab = ShellTab::Rules;
    let mut popup = None;
    let mut status = None;
    let mut filter = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--screenshot" => png = Some(PathBuf::from(value("--screenshot")?)),
            "--tab" => {
                let t = value("--tab")?;
                tab = ShellTab::from_arg(&t)
                    .ok_or_else(|| format!("unknown tab `{t}` (rules|config|economy|about)"))?;
            }
            "--popup" => popup = Some(value("--popup")?),
            "--status" => status = Some(value("--status")?),
            "--filter" => filter = Some(PathBuf::from(value("--filter")?)),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(png.map(|png| ShotArgs {
        png,
        tab,
        popup,
        status,
        filter,
    }))
}

/// `path` made absolute even when it does not exist yet: the nearest existing ancestor is
/// canonicalised and the rest appended.
fn canonical_lossy(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut cur = path.to_path_buf();
    loop {
        if let Ok(real) = std::fs::canonicalize(&cur) {
            return rest.iter().rev().fold(real, |acc, part| acc.join(part));
        }
        match (cur.file_name().map(|n| n.to_owned()), cur.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                cur = parent.to_path_buf();
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Pure core of [`guard_env`]: every path must lie under `temp`.
pub(crate) fn paths_under_temp(
    home: &Path,
    profile: &Path,
    game_dir: &Path,
    temp: &Path,
) -> Result<(), String> {
    let temp = canonical_lossy(temp);
    for (name, path) in [
        ("FERRISFILTER_HOME", home),
        ("USERPROFILE", profile),
        ("the game folder derived from USERPROFILE", game_dir),
    ] {
        // A `..` could climb out of temp after the check; refuse such paths outright.
        let climbs = path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir));
        if climbs || !canonical_lossy(path).starts_with(&temp) {
            return Err(format!(
                "screenshot mode refuses to run: {name} ({}) is not under the temp folder ({})",
                path.display(),
                temp.display()
            ));
        }
    }
    Ok(())
}

/// Requires `FERRISFILTER_HOME` and `USERPROFILE` (and so the derived game folder) to be under
/// the system temp folder.
pub(crate) fn guard_env() -> Result<(), String> {
    let home = std::env::var_os(crate::paths::HOME_ENV).ok_or_else(|| {
        "screenshot mode needs FERRISFILTER_HOME set to a temp folder".to_string()
    })?;
    let profile = std::env::var_os("USERPROFILE")
        .ok_or_else(|| "screenshot mode needs USERPROFILE set to a temp folder".to_string())?;
    let game_dir = game_dir()?;
    paths_under_temp(
        Path::new(&home),
        Path::new(&profile),
        &game_dir,
        &std::env::temp_dir(),
    )
}

fn game_dir() -> Result<PathBuf, String> {
    crate::dynamic::default_destination()
        .ok_or_else(|| "screenshot mode needs USERPROFILE set".to_string())
}

/// Copies `--filter` into the temp game folder as `dynamic.filter`. Call only after
/// [`guard_env`] succeeded.
pub(crate) fn stage_filter(args: &ShotArgs) -> Result<(), String> {
    let Some(src) = &args.filter else {
        return Ok(());
    };
    let dir = game_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let dest = dir.join("dynamic.filter");
    std::fs::copy(src, &dest)
        .map(|_| ())
        .map_err(|e| format!("cannot copy {} to {}: {e}", src.display(), dest.display()))
}

/// Saves a captured frame as a PNG.
pub(crate) fn save_png(image: &egui::ColorImage, path: &Path) -> Result<(), String> {
    let [w, h] = image.size;
    let mut buf = Vec::with_capacity(w * h * 4);
    for c in &image.pixels {
        buf.extend_from_slice(&c.to_srgba_unmultiplied());
    }
    image::RgbaImage::from_raw(w as u32, h as u32, buf)
        .ok_or_else(|| "the screenshot had an unexpected size".to_string())?
        .save(path)
        .map_err(|e| format!("cannot save {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Option<ShotArgs>, String> {
        parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_screenshot_flag_is_a_normal_launch() {
        assert_eq!(p(&[]), Ok(None));
    }

    #[test]
    fn every_flag_is_read() {
        let a = p(&[
            "--screenshot",
            "out.png",
            "--tab",
            "economy",
            "--popup",
            "quit",
            "--status",
            "hello",
            "--filter",
            "f.filter",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(a.png, PathBuf::from("out.png"));
        assert_eq!(a.tab, ShellTab::Economy);
        assert_eq!(a.popup.as_deref(), Some("quit"));
        assert_eq!(a.status.as_deref(), Some("hello"));
        assert_eq!(a.filter, Some(PathBuf::from("f.filter")));
        let b = p(&["--screenshot", "o.png"]).unwrap().unwrap();
        assert_eq!(b.tab, ShellTab::Rules);
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(p(&["--nope"]).is_err());
        assert!(p(&["--screenshot"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--tab", "x"]).is_err());
    }

    #[test]
    fn only_temp_paths_pass_the_guard() {
        let temp = std::env::temp_dir();
        let ok_home = temp.join("ff-shot-test/home");
        let ok_profile = temp.join("ff-shot-test/profile");
        let ok_game = ok_profile.join("OneDrive/Documentos/My Games/Path of Exile 2");
        assert!(paths_under_temp(&ok_home, &ok_profile, &ok_game, &temp).is_ok());

        // A real-looking profile outside temp is refused, naming the variable.
        let real = Path::new("C:/Users/someone");
        let real_game = real.join("OneDrive/Documentos/My Games/Path of Exile 2");
        let err = paths_under_temp(&ok_home, real, &real_game, &temp).unwrap_err();
        assert!(err.contains("USERPROFILE"), "{err}");
        let err = paths_under_temp(real, &ok_profile, &ok_game, &temp).unwrap_err();
        assert!(err.contains("FERRISFILTER_HOME"), "{err}");
        // A game folder that escapes temp via `..` is refused.
        let sneaky = ok_profile.join("../../../Windows");
        assert!(paths_under_temp(&ok_home, &ok_profile, &sneaky, &temp).is_err());
    }
}
