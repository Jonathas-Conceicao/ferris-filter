//! `ff` binary — short alias for `ferrisfilter`. Thin shim over `ferrisfilter::run`;
//! identical behavior to the `ferrisfilter` binary.

fn main() -> miette::Result<()> {
    ferrisfilter::run()
}
