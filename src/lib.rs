// FerrisFilter library crate.
//
// Exposes all modules so that integration tests and other binaries can call
// sync/generate logic directly via `ferrisfilter::sync::*` etc., without any
// live network access.
//
// The dual [[bin]] targets (ferrisfilter + ff) both compile src/main.rs which
// re-uses these modules via `use ferrisfilter::...`.

pub mod build_info;
pub mod cli;
pub mod compile;
pub mod dynamic;
pub mod emitter;
pub mod error;
pub mod filter;
pub mod install;
pub mod loader;
pub mod overlay;
pub mod paths;
pub mod run;
pub mod safe_write;
pub mod settings;
pub mod skin;
pub mod styled;
pub use run::run;
pub mod models;
pub mod pricer;
pub mod sync;
