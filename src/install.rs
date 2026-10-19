// `ferrisfilter install` — copy a generated .filter into the PoE2 config folder so it
// takes effect in-game. This is the native replacement for the old install.bat: it copies
// an already-generated filter (run `ferrisfilter generate` first) — it does not regenerate.
//
// Default destination is the game folder, resolved the same way as in the overlay
// (FERRISFILTER_GAME_DIR, else the folder picked in ff, else Documents\My Games\Path of
// Exile 2); pass --dest to override.

use std::path::{Path, PathBuf};

/// Copy `input` into the destination PoE2 config folder, keeping the source file name
/// (so the default `universal.filter` lands as `<dest>/universal.filter`).
///
/// Mirrors install.bat's guards: errors (non-zero exit) if the source filter is missing
/// or the destination folder does not exist, rather than silently creating paths.
pub fn cmd_install(
    input: &str,
    dest: Option<String>,
    saved_game_folder: Option<&Path>,
) -> miette::Result<()> {
    let src = Path::new(input);
    if !src.is_file() {
        return Err(miette::miette!(
            "Source filter not found: `{input}` — run `ferrisfilter generate` first"
        ));
    }

    let dest_dir = match dest {
        Some(d) => PathBuf::from(d),
        None => crate::dynamic::default_destination(saved_game_folder).ok_or_else(|| {
            miette::miette!(
                "Cannot find the Documents folder; set FERRISFILTER_GAME_DIR or pass --dest <folder>."
            )
        })?,
    };

    if !dest_dir.is_dir() {
        return Err(miette::miette!(
            "Destination folder not found: `{}` — make sure Path of Exile 2 has been run \
             at least once, or pass --dest <folder>.",
            dest_dir.display()
        ));
    }

    let file_name = src
        .file_name()
        .ok_or_else(|| miette::miette!("Invalid source path: `{input}`"))?;
    let dest_path = dest_dir.join(file_name);

    std::fs::copy(src, &dest_path).map_err(|e| {
        miette::miette!("Failed to copy `{input}` to `{}`: {e}", dest_path.display())
    })?;

    println!("installed {input} to {}", dest_path.display());
    Ok(())
}
