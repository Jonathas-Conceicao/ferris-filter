# Phase 29: Workspace & Asset Tool from the Root - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Turn the repo root into one Cargo workspace. `ff` (package `ferrisfilter`) stays the root/default
package; `tools/ff-art` becomes a workspace member runnable from the root. No UI work.

</domain>

<decisions>
## Implementation Decisions

### Workspace layout
- Root `Cargo.toml` gains `[workspace]` with `members = [".", "tools/ff-art"]` (or equivalent) and
  `default-members = ["."]`, so `cargo build` / `cargo run` / `cargo test` at the root build only
  `ff` and never compile `ff-art` or its heavy deps (poe_data_tools, resvg, image-extras).
- Remove the standalone `[workspace]` table from `tools/ff-art/Cargo.toml` and its own
  `tools/ff-art/Cargo.lock`; one lockfile at the root.
- `cargo run -p ff-art -- <cmd>` works from the root, plus a `cargo ff-art` alias in
  `.cargo/config.toml` (`[alias] ff-art = "run -p ff-art --release --"` or similar).
- ff-art resolves its default paths (`assets/poe2/`, `assets/poe2/manifest.toml`,
  `assets/ferris/`, `assets/ui/`) against the workspace/repo root (e.g. via `CARGO_MANIFEST_DIR`
  at compile time + `../..`, or by walking up to the workspace root), never the working directory,
  so it works from any cwd.

### Lockfile
- **Commit `Cargo.lock`** (author decision): remove it from `.gitignore` and commit the root lock
  as part of this rework. Carry ff-art's previously pinned versions over where possible.

### Toolchain
- **Everything on nightly** (author decision): add `rust-toolchain.toml` at the repo root pinning
  `channel = "nightly"` (components rustfmt, clippy). The author's machine default is already
  nightly (`rustup default` = nightly-x86_64-pc-windows-msvc, verified 2026-10-16).
- Update README / CLAUDE.md wording that says ff-art is "its own workspace" / "cd tools/ff-art".

### Claude's Discretion
- Exact alias name/flags, path-resolution mechanism, whether shared deps go in
  `[workspace.dependencies]`.

</decisions>

<code_context>
## Existing Code Insights

- `Cargo.toml`: package `ferrisfilter` 0.5.0, edition 2024, `default-run = "ff"`, bins
  `ferrisfilter`, `ff`, `ff-cli`, `ff-proto` (ff-proto removed in Phase 35, not here).
- `tools/ff-art/Cargo.toml`: own `[workspace]`, deps poe_data_tools 4.0.1 (nightly-only), image,
  image-extras (dds), resvg 0.45, clap, serde, toml, anyhow. Subcommands extract (default), list,
  dump, sheet, info, zoom, ferris; `--game <dir>` default Steam folder.
- `.gitignore` line 6 ignores `Cargo.lock`.
- Tests must stay green; fixtures byte-identical; `cargo fmt --check` clean.

</code_context>

<specifics>
## Specific Ideas

- The author's complaint: "I had to cd somewhere to run the crate" — the fix must make
  `cargo ff-art` (and `cargo run -p ff-art`) work from the repo root.

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
