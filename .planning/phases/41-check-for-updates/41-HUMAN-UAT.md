---
status: partial
phase: 41-check-for-updates
source: [41-VERIFICATION.md, 41-MANUAL-VERIFICATION.md]
started: 2026-10-18T00:00:00Z
updated: 2026-10-18T00:00:00Z
---

## Current Test

[awaiting human testing - deferred by the author to the end of the v0.7 run; needs a published
final release newer than the running build (releases/latest answers 404 today). Steps and exact
expected texts are in 41-MANUAL-VERIFICATION.md.]

## Tests

### 1. Check, download, ready
expected: Click Check for updates: status `Checking for updates…`, button `Downloading <ver>…`, then status `<ver> available, released <date>` and button `Install <ver> and quit`
result: [pending]

### 2. Install, nothing unapplied
expected: Overlay exits at once, installer wizard completes with no UAC or SmartScreen prompt, Launch FerrisFilter reopens ff
result: [pending]

### 3. Install, unapplied edits
expected: Unapplied changes popup opens; Cancel keeps `Install <ver> and quit` and a later tray Quit launches no installer; Quit without applying exits and starts the wizard
result: [pending]

### 4. Already current
expected: `Up to date (<ver>, released <date>)`
result: [pending]

### 5. Offline
expected: Warning `Could not reach GitHub. Check your connection and try again.`
result: [pending]

### 6. Quit cleanup
expected: Quitting while Downloading, or while Ready without Install, leaves no `%TEMP%\FerrisFilter-update-*` folder
result: [pending]

### 7. GitHub field formats
expected: Real `published_at` is `YYYY-MM-DDTHH:MM:SSZ` and each asset `browser_download_url` matches the accepted URL pattern
result: [pending]

### 8. Rate limit (optional)
expected: Warning `GitHub is limiting requests. Try again in a few minutes.`
result: [pending]

## Summary

total: 8
passed: 0
issues: 0
pending: 8
skipped: 0
blocked: 0

## Gaps
