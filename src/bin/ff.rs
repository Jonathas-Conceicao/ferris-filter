//! `ff` binary — the GUI overlay entry point. In release builds this runs
//! under the Windows GUI subsystem so no console window flashes behind the
//! window; debug builds keep a console so panics and `eprintln!` stay visible
//! (see `13-RESEARCH.md` Pitfall 5). The console CLI now lives under
//! `ff-cli`/`ferrisfilter` (see `src/bin/ff-cli.rs`).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    ferrisfilter::overlay::launch()
}
