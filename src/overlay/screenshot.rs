//! Debug-only screenshot mode for `ff`: `ff --screenshot <png> [--tab t] [--popup p] [--status s]
//! [--filter f] [--expand n] [--expand-economy n] [--search text] [--rename-section n] [--expand-style name]
//! [--picker text|border|background|rule] [--dropdown sound] [--tier-text TIER=TEXT] [--stage-economy add|remove]` renders the
//! shell once, saves the frame as a PNG and exits.
//!
//! The whole module is compiled out of release builds. The mode skips the single-instance guard,
//! the hotkey and the tray, never applies, reloads or syncs, and refuses to run unless
//! `FERRISFILTER_HOME`, `USERPROFILE` and the game folder derived from it all live under the
//! system temp directory, which must itself have a `Temp`/`tmp` component (`TMP`/`TEMP` are
//! environment variables too) -- so it can never read or write the real game folder or the user's
//! live `styles.json` / `tiers.json` / `prices.json`. `--filter` is staged under the filter name
//! the temp home selects and never overwrites a file in a game folder this mode did not create.

use std::path::{Path, PathBuf};

use super::shell::ShellTab;

/// The popups `--popup` can open.
pub(crate) const POPUPS: [&str; 8] = [
    "changed",
    "switch",
    "quit",
    "delete-rule",
    "delete-style",
    "remove-economy",
    "discard-config",
    "discard-rules",
];

/// The colour pickers `--picker` can open (the `colour_popup` tags).
pub(crate) const PICKERS: [&str; 4] = ["text", "border", "background", "rule"];

/// The dropdowns `--dropdown` can open: `sound` is the first drawn style editor's Alert sound
/// list (expand a style or a Custom rule too).
pub(crate) const DROPDOWNS: [&str; 1] = ["sound"];

/// The parsed command line of the screenshot mode.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShotArgs {
    /// Where the PNG goes.
    pub png: PathBuf,
    /// The tab to show.
    pub tab: ShellTab,
    /// A popup to open before the first frame (one of [`POPUPS`]).
    pub popup: Option<String>,
    /// A status line text to show in place of the computed one (to see truncation).
    pub status: Option<String>,
    /// A filter file copied into the temp game folder as `dynamic.filter` before the app starts.
    pub filter: Option<PathBuf>,
    /// Expand the n-th rule (1-based) of the RULES tab.
    pub expand: Option<usize>,
    /// Open the economy section and expand its n-th rule (1-based), shown read-only.
    pub expand_economy: Option<usize>,
    /// Stage the economy section add or remove (`add` or `remove`) before the first frame, as the
    /// Economy tab button does; nothing is written.
    pub stage_economy: Option<String>,
    /// Text for the RULES search box.
    pub search: Option<String>,
    /// Open the n-th user section header (1-based, economy skipped) in rename mode.
    pub rename_section: Option<usize>,
    /// Expand this style in the CONFIG tab's Styles tables.
    pub expand_style: Option<String>,
    /// Open the colour picker with this tag (one of [`PICKERS`]).
    pub picker: Option<String>,
    /// Open this dropdown (one of [`DROPDOWNS`]).
    pub dropdown: Option<String>,
    /// Seed this tier cutoff text on ECONOMY, e.g. `S=abc`, to show the inline validation.
    pub tier_text: Option<(String, String)>,
}

/// Parses the arguments after the program name. `Ok(None)` when `--screenshot` is absent: a
/// normal launch, which ignores its arguments like a release `ff` does (review WR-08). With
/// `--screenshot`, an unknown flag or a missing value is an `Err`.
pub(crate) fn parse(args: impl Iterator<Item = String>) -> Result<Option<ShotArgs>, String> {
    let args: Vec<String> = args.collect();
    if !args.iter().any(|a| a == "--screenshot") {
        return Ok(None);
    }
    let mut png = None;
    let mut tab = ShellTab::Rules;
    let mut popup = None;
    let mut status = None;
    let mut filter = None;
    let mut expand = None;
    let mut expand_economy = None;
    let mut stage_economy = None;
    let mut search = None;
    let mut rename_section = None;
    let mut expand_style = None;
    let mut picker = None;
    let mut dropdown = None;
    let mut tier_text = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--screenshot" => png = Some(PathBuf::from(value("--screenshot")?)),
            "--tab" => {
                let t = value("--tab")?;
                tab = ShellTab::from_arg(&t)
                    .ok_or_else(|| format!("unknown tab `{t}` (rules|config|economy|about)"))?;
            }
            "--popup" => {
                let p = value("--popup")?;
                if !POPUPS.contains(&p.as_str()) {
                    return Err(format!("unknown popup `{p}` ({})", POPUPS.join("|")));
                }
                popup = Some(p);
            }
            "--status" => status = Some(value("--status")?),
            "--filter" => filter = Some(PathBuf::from(value("--filter")?)),
            "--expand" => {
                let n = value("--expand")?;
                expand = match n.parse::<usize>() {
                    Ok(v) if v >= 1 => Some(v),
                    _ => return Err(format!("--expand needs a rule number >= 1, got `{n}`")),
                };
            }
            "--expand-economy" => {
                let n = value("--expand-economy")?;
                expand_economy = match n.parse::<usize>() {
                    Ok(v) if v >= 1 => Some(v),
                    _ => {
                        return Err(format!(
                            "--expand-economy needs a rule number >= 1, got `{n}`"
                        ));
                    }
                };
            }
            "--stage-economy" => {
                let v = value("--stage-economy")?;
                if v != "add" && v != "remove" {
                    return Err(format!("--stage-economy needs add or remove, got `{v}`"));
                }
                stage_economy = Some(v);
            }
            "--rename-section" => {
                let n = value("--rename-section")?;
                rename_section = match n.parse::<usize>() {
                    Ok(v) if v >= 1 => Some(v),
                    _ => {
                        return Err(format!(
                            "--rename-section needs a section number >= 1, got `{n}`"
                        ));
                    }
                };
            }
            "--expand-style" => expand_style = Some(value("--expand-style")?),
            "--picker" => {
                let p = value("--picker")?;
                if !PICKERS.contains(&p.as_str()) {
                    return Err(format!("unknown picker `{p}` ({})", PICKERS.join("|")));
                }
                picker = Some(p);
            }
            "--dropdown" => {
                let d = value("--dropdown")?;
                if !DROPDOWNS.contains(&d.as_str()) {
                    return Err(format!("unknown dropdown `{d}` ({})", DROPDOWNS.join("|")));
                }
                dropdown = Some(d);
            }
            "--tier-text" => {
                let t = value("--tier-text")?;
                tier_text = match t.split_once('=') {
                    Some((tier, text)) if !tier.is_empty() => {
                        Some((tier.to_string(), text.to_string()))
                    }
                    _ => return Err(format!("--tier-text needs TIER=TEXT, got `{t}`")),
                };
            }
            "--search" => search = Some(value("--search")?),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(png.map(|png| ShotArgs {
        png,
        tab,
        popup,
        status,
        filter,
        expand,
        expand_economy,
        stage_economy,
        search,
        rename_section,
        expand_style,
        picker,
        dropdown,
        tier_text,
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

/// Whether `temp` looks like a real temp folder: one of its components is named `Temp` or
/// `tmp` (any case). `std::env::temp_dir()` comes from the same kind of environment variable
/// (`TMP`/`TEMP`) the guard distrusts, so `TEMP=C:\Users\me` or `TEMP=C:\` must not make the
/// real profile, and with it the real game folder, count as "under temp" (review WR-07).
pub(crate) fn temp_looks_real(temp: &Path) -> bool {
    temp.components().any(|c| match c {
        std::path::Component::Normal(name) => name
            .to_str()
            .is_some_and(|n| n.eq_ignore_ascii_case("temp") || n.eq_ignore_ascii_case("tmp")),
        _ => false,
    })
}

/// Pure core of [`guard_env`]: every path must lie under `temp`, which must itself look like a
/// temp folder ([`temp_looks_real`]).
pub(crate) fn paths_under_temp(
    home: &Path,
    profile: &Path,
    game_dir: &Path,
    temp: &Path,
) -> Result<(), String> {
    let temp = canonical_lossy(temp);
    if !temp_looks_real(&temp) {
        return Err(format!(
            "screenshot mode refuses to run: the temp folder ({}) has no Temp or tmp component; \
             check TMP/TEMP",
            temp.display()
        ));
    }
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
    let home = std::env::var_os(crate::locations::HOME_ENV).ok_or_else(|| {
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

/// The marker file the screenshot mode leaves in a game folder it created, so later runs may
/// overwrite the filter they staged there.
const STAGED_MARKER: &str = ".ff-screenshot-staged";

/// Copies `--filter` into the temp game folder under the filter name the temp home's
/// `ff-settings.json` selects (default `dynamic.filter`), which is the file the app then loads.
/// Call only after [`guard_env`] succeeded.
pub(crate) fn stage_filter(args: &ShotArgs) -> Result<(), String> {
    let Some(src) = &args.filter else {
        return Ok(());
    };
    let home = std::env::var_os(crate::locations::HOME_ENV)
        .ok_or_else(|| "screenshot mode needs FERRISFILTER_HOME set".to_string())?;
    let (settings, _) = crate::settings::load(Path::new(&home));
    stage_into(src, &game_dir()?, settings.filter_name())
}

/// Copies `src` to `dir/name`. A game folder this mode did not create (no [`STAGED_MARKER`])
/// never has an existing file overwritten (review WR-07).
fn stage_into(src: &Path, dir: &Path, name: &str) -> Result<(), String> {
    let marker = dir.join(STAGED_MARKER);
    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        std::fs::write(&marker, b"")
            .map_err(|e| format!("cannot write {}: {e}", marker.display()))?;
    }
    let dest = dir.join(name);
    if dest.exists() && !marker.exists() {
        return Err(format!(
            "screenshot mode refuses to overwrite {}: its folder was not created by screenshot mode",
            dest.display()
        ));
    }
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
        // Other arguments (a shortcut, a launcher) do not stop a normal launch.
        assert_eq!(p(&["--nope"]), Ok(None));
        assert_eq!(p(&["foo", "--tab", "x"]), Ok(None));
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
            "--expand",
            "3",
            "--expand-economy",
            "2",
            "--stage-economy",
            "add",
            "--search",
            "ring",
            "--rename-section",
            "2",
            "--expand-style",
            "S",
            "--picker",
            "text",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(a.rename_section, Some(2));
        assert_eq!(a.expand_style.as_deref(), Some("S"));
        assert_eq!(a.picker.as_deref(), Some("text"));
        assert_eq!(a.dropdown, None);
        for name in DROPDOWNS {
            let d = p(&["--screenshot", "o.png", "--dropdown", name])
                .unwrap()
                .unwrap();
            assert_eq!(d.dropdown.as_deref(), Some(name));
        }
        for slot in PICKERS {
            assert!(p(&["--screenshot", "o.png", "--picker", slot]).is_ok());
        }
        assert_eq!(a.expand, Some(3));
        assert_eq!(a.expand_economy, Some(2));
        assert_eq!(a.stage_economy.as_deref(), Some("add"));
        assert_eq!(a.search.as_deref(), Some("ring"));
        assert_eq!(a.png, PathBuf::from("out.png"));
        assert_eq!(a.tab, ShellTab::Economy);
        assert_eq!(a.popup.as_deref(), Some("quit"));
        assert_eq!(a.status.as_deref(), Some("hello"));
        assert_eq!(a.filter, Some(PathBuf::from("f.filter")));
        let b = p(&["--screenshot", "o.png"]).unwrap().unwrap();
        assert_eq!(b.tab, ShellTab::Rules);
    }

    #[test]
    fn every_popup_name_parses() {
        for name in POPUPS {
            let a = p(&["--screenshot", "o.png", "--popup", name])
                .unwrap()
                .unwrap();
            assert_eq!(a.popup.as_deref(), Some(name));
        }
        assert!(POPUPS.contains(&"discard-rules"));
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(p(&["--screenshot", "o.png", "--nope"]).is_err());
        assert!(p(&["--screenshot"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--tab", "x"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--popup", "nope"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--expand", "0"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--expand-economy", "0"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--expand-economy"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--stage-economy", "x"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--stage-economy"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--expand", "x"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--expand"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--search"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--rename-section", "0"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--rename-section", "x"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--rename-section"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--picker", "foo"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--picker"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--dropdown", "foo"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--dropdown"]).is_err());
        assert!(p(&["--screenshot", "o.png", "--expand-style"]).is_err());
        let e = p(&["--screenshot", "o.png", "--tier-text", "noequals"]).unwrap_err();
        assert!(e.contains("--tier-text needs TIER=TEXT"), "{e}");
        assert!(p(&["--screenshot", "o.png", "--tier-text", "=x"]).is_err());
    }

    #[test]
    fn tier_text_is_read() {
        let a = p(&["--screenshot", "o.png", "--tier-text", "S=abc"])
            .unwrap()
            .unwrap();
        assert_eq!(a.tier_text, Some(("S".into(), "abc".into())));
        let b = p(&["--screenshot", "o.png"]).unwrap().unwrap();
        assert_eq!(b.tier_text, None);
    }

    #[test]
    fn a_temp_folder_without_a_temp_component_is_refused() {
        assert!(temp_looks_real(Path::new("C:/Users/me/AppData/Local/Temp")));
        assert!(temp_looks_real(Path::new("/tmp")));
        assert!(temp_looks_real(Path::new(
            "C:/Users/me/AppData/Local/Temp/2"
        )));
        assert!(!temp_looks_real(Path::new("C:/Users/me")));
        assert!(!temp_looks_real(Path::new("C:/")));
        // TEMP pointed at the profile: the real game folder would otherwise pass.
        let profile = Path::new("C:/Users/me");
        let game = profile.join("OneDrive/Documentos/My Games/Path of Exile 2");
        let err = paths_under_temp(&profile.join("ff"), profile, &game, profile).unwrap_err();
        assert!(err.contains("TEMP"), "{err}");
    }

    #[test]
    fn staging_never_overwrites_a_file_in_a_folder_it_did_not_create() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.filter");
        std::fs::write(&src, "Show\n").unwrap();

        // A folder this mode creates is marked, and later runs may overwrite the staged file.
        let fresh = tmp.path().join("fresh");
        stage_into(&src, &fresh, "x.filter").unwrap();
        assert!(fresh.join(STAGED_MARKER).exists());
        std::fs::write(&src, "Hide\n").unwrap();
        stage_into(&src, &fresh, "x.filter").unwrap();
        assert_eq!(
            std::fs::read_to_string(fresh.join("x.filter")).unwrap(),
            "Hide\n"
        );

        // A pre-existing folder: a new file is fine, an existing one is never replaced.
        let existing = tmp.path().join("existing");
        std::fs::create_dir_all(&existing).unwrap();
        std::fs::write(existing.join("dynamic.filter"), "mine").unwrap();
        assert!(stage_into(&src, &existing, "dynamic.filter").is_err());
        assert_eq!(
            std::fs::read_to_string(existing.join("dynamic.filter")).unwrap(),
            "mine"
        );
        stage_into(&src, &existing, "other.filter").unwrap();
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
