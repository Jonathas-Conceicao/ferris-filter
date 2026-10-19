# Phase 41 — Check for updates: Manual Verification (deferred)

**Status:** deferred, non-blocking. These checks need a real published, non-pre-release GitHub
release newer than the running build, and a real installer. At the time of writing
`releases/latest` answers 404, so none of them can run yet. Automated tests cover every code path
with a fake GitHub (the `UpdateHttp` seam) and temp folders; nothing here blocks the phase.

**Before you start:** publish a final release `v<X.Y.Z>` with `FerrisFilter-<X.Y.Z>-setup.exe` and
`SHA256SUMS.txt`, then run a build whose Cargo version is lower than that tag. Use a release build
(`cargo build --release --bin ff`) and open the About tab. Do not run these against a build whose
live `styles.json` / `tiers.json` you care about without a `FERRISFILTER_HOME` copy.

| # | Check | Steps | Expected | Result |
|---|-------|-------|----------|--------|
| 1 | Check, download, ready | Click **Check for updates** | Status `Checking for updates…`, then the button reads `Downloading <ver>…`, then the status reads `<ver> available, released <date>` and the button `Install <ver> and quit` | |
| 2 | Install, nothing unapplied | With no unapplied edits click **Install <ver> and quit** | The overlay exits at once, the installer wizard appears and completes with no UAC or SmartScreen prompt, **Launch FerrisFilter** reopens ff | |
| 3 | Install, unapplied edits | Make a rule edit, click Install | The **Unapplied changes** popup opens. **Cancel** keeps the button on `Install <ver> and quit` and a later tray Quit launches no installer. **Quit without applying** exits and starts the wizard | |
| 4 | Already current | Run a build whose version equals the latest tag, click Check | `Up to date (<ver>, released <date>)` | |
| 5 | Offline | Turn on airplane mode, click Check | The warning `Could not reach GitHub. Check your connection and try again.` | |
| 6 | Quit cleanup | Quit while `Downloading…`; separately quit while Ready (no Install click) | No `%TEMP%\FerrisFilter-update-*` folder is left in either case | |
| 7 | GitHub field formats | Inspect the real `releases/latest` JSON | `published_at` is `YYYY-MM-DDTHH:MM:SSZ` and each asset `browser_download_url` matches the accepted URL pattern (RESEARCH A3, the decision 5 URL check) | |
| 8 | Rate limit (optional) | Click Check more than 60 times in an hour without a token | The warning `GitHub is limiting requests. Try again in a few minutes.` | |

Record each Result (pass / fail with notes) in the last column when you run them.
