---
phase: 41
slug: check-for-updates
status: approved
reviewed_at: 2026-10-18
shadcn_initialized: false
preset: none
created: 2026-10-18
---

# Phase 41 - UI Design Contract

> Visual and interaction contract. FerrisFilter is a Rust egui overlay with the PoE2 game skin
> (`src/skin/`); there is no web design system. Every value below is an existing skin widget,
> palette constant or font. Sources: 41-CONTEXT.md (locked), 39-UI-SPEC.md (format, status-line
> conventions), 40-01/40-03/40-05 plans (`ff --quit`, `USER_AGENT` in `src/sync/client.rs`,
> `SHA256SUMS.txt`), CLAUDE.md overlay notes, `src/overlay/about_view.rs` (`about_tab`),
> `src/overlay/shell.rs` (`last_sync_text`, `relative_sync_text`), `src/skin/widgets.rs`
> (`ff_status`, `StatusKind`), `src/overlay/apply.rs` (`quit_step`), `src/overlay/instance.rs`
> (`MessageBoxW`), memory note "overlay UI not texty".

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (egui + hand-built skin, not shadcn) |
| Preset | not applicable |
| Component library | `src/skin/` `ff_*` widgets over egui |
| Icon library | none added |
| Font | Fontin Regular (`fonts::regular`), Fontin SmallCaps (`fonts::small_caps`) |

Registry safety: not applicable (no shadcn, no third-party registries).

---

## Scope of UI change

| Surface | Change |
|---------|--------|
| About tab | One new centred button under the version line (above the repository link). `about_tab` gains state and returns an action (`None` / `Check` / `InstallAndQuit`). |
| Status line | New event messages (check outcome, failures). No new standing condition. |
| Quit path | The button reuses the tray Quit sequence (`quit_step`, Unapplied changes popup); no new popup. |
| Failure after exit began | One Windows message box (existing `MessageBoxW` mechanism). |
| Debug screenshot mode | New fake-update flag (below). |

No new tab, popup, widget, colour, font or size. No standing explanatory text is added to the
About page; explanations are tooltips.

---

## Spacing Scale

Existing skin constants and the About page's own rhythm; nothing new.

| Token | Value | Usage |
|-------|-------|-------|
| Version to button gap | 8pt | `add_space(8.0)` after the version label |
| Button to repo link gap | 8pt | `add_space(8.0)` before the hyperlink (replaces the 4pt used without a button) |
| Section gap | 16pt | unchanged gaps around the Ferris image and divider |
| Button size | skin button art height, width 240pt | see below |

Exceptions: the button height is the skin's existing button art height (off the 4pt grid, as in
Phases 37 to 39).

---

## Typography

No new sizes.

| Role | Font / Size | Usage |
|------|-------------|-------|
| Button caption | `small_caps(17)` (`ff_button_ex`) | all four button states |
| Version line | `regular(14)` `MUTED` | unchanged |
| Status line | `regular(15)` in the kind's colour (`status_job`) | all messages below |
| Tooltip | egui default tooltip | button hover text |

---

## Color

Palette constants (`src/skin/palette.rs`) only.

| Role | Constant | Usage here |
|------|----------|------------|
| Dominant | `GROUND` / `PANEL` | window surface |
| Secondary | button art | the button, as every other `ff_button` |
| Accent (gold) | `GOLD` | the existing repository link only; the new button adds none |
| Status colours | `StatusKind::color()`: Info `MUTED`, Ok `OK`, Warning `WARN`, Error `ERROR` | per message table |
| Disabled | `DISABLED` | button caption while Checking / Downloading |

Accent reserved for: the repository hyperlink (existing). The "Install <version> and quit" button
is not gold or red: it is the page's primary action by position and by the status message that
accompanies it. Destructive red is not used (installing is not destructive; the Unapplied changes
popup owns the safety).

---

## About tab layout (top to bottom)

Unchanged down to the version line: `FerrisFilter`, description, orange Ferris, `Version x.y`.
Then:

| # | Element | Notes |
|---|---------|-------|
| 1 | Update button | centred, `ff_button_ex(ui, skin, caption, 240.0, enabled, false)`; the About column is `Align::Center` |
| 2 | Repository link | unchanged |
| 3 | Divider, footer | unchanged (footer stays pinned to the bottom) |

The result of the check is NOT drawn on the page: it goes to the status line (one place, no
duplicate text). The cramped-page test (300pt tall) must still render: the button adds about 40pt;
the footer is bottom-anchored and may overlap on a very short window exactly as the page already
allows.

---

## Update button: states and copy

One button, one caption per state; the state is About-tab view state, never persisted.

| State | Caption | Enabled | Click |
|-------|---------|---------|-------|
| Idle (start, after up to date, after any failure) | `Check for updates` | yes | starts the check worker |
| Checking | `Checking…` | no | none |
| Downloading (newer found; installer and sums being fetched and verified) | `Downloading…` | no | none |
| Ready (installer verified) | `Install <version> and quit` (e.g. `Install 0.8.0 and quit`) | yes | install and quit sequence below |

- `<version>` is the release tag without a leading `v`.
- Button tooltips: Idle `Look for a newer version on GitHub.`; Checking `Asking GitHub.`;
  Downloading `Downloading and checking the installer.`; Ready `Closes FerrisFilter and starts the installer.`
  The disabled states show theirs through the disabled hover, as other disabled controls do.
- Ready is sticky: it persists while the user visits other tabs. There is no "Check again" while
  Ready (one result at a time). It ends by installing, by a failed installer re-check (below), or
  by restarting the overlay.
- Cancel in the **Unapplied changes** popup (`QuitChoice::Cancel`, `QuitStep::CloseConfirm`) leaves
  the button in Ready; nothing is lost and the installer is kept.
- The tray Quit entry and the window close button are unchanged and unrelated.

### Install and quit sequence (behaviour contract; CONTEXT.md wins)

1. On click, re-check that the verified installer still exists in its `%TEMP%` folder. If it is
   gone: status `Error` `The installer for <version> is no longer there. Check for updates again.`,
   the overlay stays open, the button returns to `Check for updates`.
2. Otherwise the click goes through the normal Quit path (`quit_step` with `QuitEvent::TrayQuit`):
   with nothing unapplied it is `QuitNow`; with unapplied changes the existing **Unapplied
   changes** popup opens (`Apply & quit` / `Quit without applying` / `Cancel`).
3. When the quit proceeds, the shutdown releases the hotkey, the tray icon and the instance name
   (the existing order), then spawns the installer (normal visible wizard, no `/SILENT`), then
   exits. The installer's own `ff --quit` step (Phase 40) finds nothing running.
4. If the spawn fails after the release, the window is already gone and the status line cannot be
   seen, so a Windows message box (the `MessageBoxW` path `instance.rs` already uses) shows
   `Could not start the FerrisFilter installer: <cause>. It was saved to <path>.` and the process
   exits. `<path>` is the full path of the verified installer.
5. Quit while Downloading (before Ready): the overlay quits normally, the partial download is
   deleted and no installer is launched.

---

## Status line messages

All are event messages (shown until the next edit, per Phase 30), set through the existing status
event path. None is a standing condition, so the priority order is unchanged.

Date format: local `YYYY-MM-DD` (the release's `published_at` converted with the same local UTC
offset helper the Economy tab uses). Absolute, not relative: a release date is a calendar fact the
user compares with patch notes, and "3 d ago" would go stale while the message stays on screen.
`ff_status` has no per-message tooltip (it only shows the full text when the line was cut), so no
date tooltip is added; the date in the message is enough.

| Situation | Text | Kind |
|-----------|------|------|
| Check started | `Checking for updates…` | Info |
| Up to date | `Up to date (<version>, released <date>)` e.g. `Up to date (0.7.0, released 2026-10-18)` | Ok |
| Up to date, repo has no releases yet (HTTP 404) | `Up to date (<version>)` | Ok |
| Newer found, downloading | `Downloading <version>…` | Info |
| Newer found, verified | `<version> available, released <date>` e.g. `0.8.0 available, released 2026-11-02` | Info |
| Offline, DNS or timeout (transport error) | `Could not reach GitHub. Check your connection and try again.` | Warning |
| Rate limited (HTTP 403 or 429 with rate-limit headers) | `GitHub is limiting requests. Try again in a few minutes.` | Warning |
| Release has no installer asset | `Release <version> has no installer. Download it from the repository page.` | Error |
| `SHA256SUMS.txt` missing | `Release <version> has no checksum file, so it was not downloaded.` | Error |
| `SHA256SUMS.txt` invalid (no line for the installer, or unparseable) | `The checksum file for <version> is not valid, so it was not downloaded.` | Error |
| Checksum mismatch | `The download of <version> did not match its checksum and was deleted.` | Error |
| Download failed (connection drop, HTTP error, disk) | `Could not download <version>. Try again.` | Error |
| Verified installer gone at click time | `The installer for <version> is no longer there. Check for updates again.` | Error |
| Any other non-2xx status, or a tag that is not valid semver | `Could not read the latest version from GitHub.` | Error |

Rules:
- A failure message always means: any partial or unverified file deleted, nothing else changed,
  button back to `Check for updates`. A failed installer start is not a status message (see the
  message box in the sequence above).
- The transition is Checking, then Downloading, then Ready, with the status text following. A dev
  build (`0.7.0-dev`) compares on its base version `0.7.0`; its up-to-date message shows that
  base version.
- A tag older than the running build counts as up to date, same text.
- Every message is one line (`ff_status` cuts with an ellipsis and shows the full text on hover by
  its existing rule), no emojis, no exclamation marks.
- Warning is for transient problems the user fixes by waiting or reconnecting; Error is for
  anything wrong with the release or the download.
- `available` appears only after the checksum passed.

Network notes for the executor (no UI effect): requests use the shared `USER_AGENT` from
`src/sync/client.rs`; `SHA256SUMS.txt` lines are `<64 hex>  <file name>` (two spaces, plain names,
setup first, written by Phase 40's `package.sh`).

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Primary CTA | `Check for updates`; when ready `Install <version> and quit` |
| Busy captions | `Checking…`, `Downloading…` |
| Empty state | not applicable (no list); the first view shows only the idle button |
| Error state | one status-line message per failure, each saying what happened and what to do next (table above) |
| Spawn failure | message box `Could not start the FerrisFilter installer: <cause>. It was saved to <path>.` |
| Destructive actions | none added. Quit with unapplied changes uses the existing **Unapplied changes** popup unchanged |

---

## Interaction states summary

| Control | Idle | Hover | Active | Disabled |
|---------|------|-------|--------|----------|
| Update button | `ff_button` art, `LABEL` caption | art hover, tooltip | press art; starts check or install | dim art, `DISABLED` caption, tooltip (Checking, Downloading) |

Accessibility: the button reports its current caption as its accessible name and its `enabled`
state; the status line reports its text as a label (existing). The check never blocks other tabs
or Apply; switching tabs mid-check is allowed and the outcome lands in the status line wherever
the user is.

Concurrency: a check or download is one worker job (Phase 39's generic worker). A Sync economy or
league fetch running does not block the check.

---

## Debug screenshot mode (executor verification; compiled out of release builds)

Existing guard applies (`FERRISFILTER_HOME`, `USERPROFILE`, game folder, settings folder all under
temp). The check and download are never started in screenshot mode; the state is injected.

New flag on `ff --screenshot <png> --tab about`:

| Flag | Effect |
|------|--------|
| `--update-state idle\|checking\|downloading\|ready:<ver>:<date>` | Sets the button state; `ready` also sets the `<version> available, released <date>` status event. |
| `--status <text>` (existing) | Used for the other messages. |

Required captures: idle; checking; downloading; ready (`ready:0.8.0:2026-11-02`, status line reads
`0.8.0 available, released 2026-11-02`); the Up to date message via `--status`; one Warning and one
Error message via `--status`; the cramped 300pt page. Pixel check: button centred on the page
column, 8pt above and below.

Unit tests (no network): version comparison (tag `v0.8.0` vs `0.7.0`, equal, older, dev base
version, invalid tag); date formatting from `published_at`; the message table for every failure
with its kind; button caption and enabled per state; 404 reads up to date; checksum line parse and
mismatch; the install click with a missing installer file.

---

## Decisions made in the author's place (non-interactive run; author may revise)

1. Date format is local `YYYY-MM-DD` in the message and no tooltip (`ff_status` supports none);
   not relative, because the message stays on screen and the release date is what users compare.
2. Status kinds: Up to date = Ok; checking, downloading and "available" = Info (available is
   information plus a button, not a fault); offline and rate limit = Warning (transient); every
   release or download defect = Error.
3. Button width 240pt, centred, between the version and the repository link; no gold, no red; the
   result is shown only in the status line (no duplicate text on the page).
4. Ready is sticky across tab switches; Cancel in the popup returns to Ready; no re-check while
   Ready.
5. "No installer" points to the repository page because a portable-zip-only release is possible.
6. Quit during Downloading just quits, deletes the partial file and launches no installer.
7. The verified installer is re-checked at click time; if gone, an Error status and the button
   returns to `Check for updates`.
8. A latest tag older than the build, or a dev build's base version equal to the tag, counts as up
   to date.
9. Error mapping: a transport error is the offline Warning; 403 or 429 with rate-limit headers is
   the rate-limit Warning; 404 is up to date without a date; any other non-2xx status is
   `Could not read the latest version from GitHub.` (Error).
10. The `--update-state` screenshot flag name is a suggestion; the executor may rename it.

---

## Checker Sign-Off

- [x] Dimension 1 Copywriting: PASS
- [x] Dimension 2 Visuals: PASS (FLAG: focal point is the orange Ferris; the update button is the page's only action)
- [x] Dimension 3 Color: PASS (FLAG: surfaces GROUND/PANEL dominant, button art secondary, gold on the link only)
- [x] Dimension 4 Typography: PASS (body line height: egui default)
- [x] Dimension 5 Spacing: PASS
- [x] Dimension 6 Registry Safety: PASS

**Approval:** approved 2026-10-18
