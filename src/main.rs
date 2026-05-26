//! `ferrisfilter` binary — a thin shim over the shared `ferrisfilter::run` entry point.
//! All logic lives in the library (see `src/run.rs`) so the `ferrisfilter` and `ff`
//! binaries can each have their own source file.

fn main() -> miette::Result<()> {
    ferrisfilter::run()
}
