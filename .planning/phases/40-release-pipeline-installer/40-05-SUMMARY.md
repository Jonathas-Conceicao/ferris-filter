---
phase: 40-release-pipeline-installer
plan: 05
subsystem: release
tags: [inno-setup, packaging, installer]
requires: [40-01, 40-02, 40-03, 40-04]
provides: [installer/ferrisfilter.iss, installer/package.sh, installer/portable-readme.txt]
key-files:
  created:
    - installer/ferrisfilter.iss
    - installer/package.sh
    - installer/portable-readme.txt
decisions:
  - AppId fixed as {1FDA92ED-8F71-4558-B482-3EC93A7AA550}
  - LICENSE.txt installed next to ff.exe (confirm at 40-07 UAT)
  - skipifsilent kept on the Launch entry; no AppMutex
  - FF_SKIP_INSTALLER=1 local switch skips ISCC
metrics:
  tasks: 2
  completed: 2026-10-18
---

# Phase 40 Plan 05: Installer script and packaging Summary

Per-user Inno Setup 6 script plus one bash packaging script (zip, installer, SHA256SUMS) shared by CI and local runs.

## Commits
- 2b05d90: feat(40-05): Inno Setup per-user installer script
- 5aa7bf0: feat(40-05): package.sh and portable readme (package.sh committed mode 100755)

## What was built
- `installer/ferrisfilter.iss`: ASCII, `PrivilegesRequired=lowest`, `{autopf}\FerrisFilter`, [Files] only ff.exe (from `/DAppExe`) and LICENSE as LICENSE.txt, `{userappdata}\FerrisFilter` with `uninsneveruninstall`, Start menu entry, unticked desktop task, ticked Launch entry, `[UninstallRun] ff.exe --quit` (hidden), `PrepareToInstall` runs `ff.exe --quit` with a Retry/Cancel loop on exit code 1.
- `installer/package.sh <version>`: stages ff.exe/LICENSE.txt/README.txt, zips with the Windows `tar.exe`, runs ISCC with `MSYS2_ARG_CONV_EXCL='*'` and the three defines, writes `dist/SHA256SUMS.txt` (setup first, then zip). `FF_EXE` overrides the exe path. ISCC is resolved before `dist/` is wiped so a missing tool fails fast.
- `installer/portable-readme.txt`: 19 lines, ASCII, no version.

## Checks run
- ASCII check on .iss and readme: 0 non-ASCII bytes. All .iss greps from the plan pass (AppId 1, forbidden words 0, 2 `Source:` lines, 1 `Flags: unchecked`, each required string once).
- `FF_EXE=/tmp/fftgt40/debug/ff.exe FF_SKIP_INSTALLER=1 bash installer/package.sh 0.0.0-local`: OK; zip lists exactly ff.exe, LICENSE.txt, README.txt; SHA256SUMS has one zip line and `sha256sum -c` passes; loud WARNING printed.
- Without the skip switch (Inno not installed): fails with `ISCC.exe not found: ...`.
- `dist/` is git-ignored (working tree shows only the orchestrator's STATE.md edit). Mode 100755 confirmed.
- NOT done: ISCC compile of the .iss (Inno Setup not installed; plan 40-07).

## Deviations from Plan
- Minor: the `.iss` header comment avoids the literal words `AppMutex` and `RestartApplications=no` so the plan's zero/one-count greps hold.
- package.sh resolves ISCC before removing dist/ (plan order had it after staging); behavior otherwise as specified.

## For Phase 41
`skipifsilent` means a `/SILENT` install does not relaunch ff. The installer runs `ff --quit` itself in PrepareToInstall (exit 0 continue, 1 Retry box, 2 or Exec failure falls back to Restart Manager). Phase 41's updater must relaunch ff itself.

## For the Release Review
- Confirm at 40-07 UAT: LICENSE.txt in the install folder (drop the second [Files] entry if declined).
- If the Unapplied changes popup is left open during uninstall, `--quit` returns 1 after its timeout and Inno may report it could not remove ff.exe (accepted, no uninstall retry loop).
- The .iss was never compiled here; first ISCC run is 40-07 or CI. Files may show LF-to-CRLF autocrlf warnings on this machine; Inno accepts either.

## Self-Check: PASSED
