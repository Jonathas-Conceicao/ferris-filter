//! `ff-cli` binary — the console CLI under a short name. Thin shim over
//! `ferrisfilter::run`; identical behavior to the `ferrisfilter` binary.
//!
//! `ff` is now the GUI overlay binary (see `src/bin/ff.rs`); this file carries
//! forward the console shim body that `ff.rs` used to hold.

fn main() -> miette::Result<()> {
    ferrisfilter::run()
}
