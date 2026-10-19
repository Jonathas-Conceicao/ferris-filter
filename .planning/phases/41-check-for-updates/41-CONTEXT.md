# Phase 41: Check for Updates - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning

<domain>
## Phase Boundary

The About tab can check GitHub Releases for a newer FerrisFilter, download and verify its installer,
and hand off to it on quit. Covers UPD-01..UPD-03. Depends on Phase 40 (published releases, stable
asset names, `SHA256SUMS.txt`, the installer and the single-instance quit path).

</domain>

<decisions>
## Implementation Decisions

### Checking
- A Check for updates button on the About tab, under the version. A click calls GitHub's
  `GET /repos/Jonathas-Conceicao/ferris-filter/releases/latest` (with a User-Agent) on a worker
  thread; that endpoint never returns drafts or pre-releases, so `-rc` builds are invisible.
- The release tag is compared with the exe's `Cargo.toml` version (semver); a dev build compares on
  its base version.
- Up to date: the status line reads "Up to date (<version>, released <date>)". A repo with no
  releases yet (404) also counts as up to date (no date then).
- Only on click: never at start-up, never in the background.

### Newer version found
- The installer `FerrisFilter-<ver>-setup.exe` and `SHA256SUMS.txt` are downloaded right away into a
  fresh folder under `%TEMP%`, and the installer's SHA-256 is checked against the sums file.
- The status line names the version and its release date ("<version> available, released <date>");
  the button becomes "Install <version> and quit".
- Install and quit goes through the normal Quit path (Unapplied changes popup: Apply & quit / Quit
  without applying / Cancel). After the overlay has released its hotkey, tray icon and instance
  name, it starts the installer (normal visible wizard) and exits.
- Portable-zip users are offered the installer too; there is no in-place exe swap.

### Failures (UPD-03)
- Offline, GitHub rate limit, a release without the installer asset, a checksum mismatch or a failed
  download: one status-line message each, any partial file deleted, nothing else changed; the button
  returns to Check for updates.

### Claude's Discretion
- Date format (follow the Economy tab's Last sync style), worker reuse (Phase 39's generic worker),
  module layout (`update.rs`), how tests fake the GitHub responses.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- reqwest 0.13 (rustls), serde_json, tempfile, `semver` (already in Cargo.lock); the generic worker
  extracted in Phase 39; the tray Quit / Unapplied changes popup path (`quit_step`) and
  `src/overlay/instance.rs`.
- `about_tab` is stateless today; it needs state and an action return.

### Established Patterns
- One-line status bar with info / ok / warning / error kinds; event messages show until the next
  edit.

### Integration Points
- About tab view, the status-line event path, the quit sequence (spawn the installer only after the
  instance name, hotkey and tray are released).

</code_context>

<specifics>
## Specific Ideas

- The author: the release date matters to users even more than the version number.

</specifics>

<deferred>
## Deferred Ideas

- Automatic or start-up update checks (out of scope for v0.7).

</deferred>
