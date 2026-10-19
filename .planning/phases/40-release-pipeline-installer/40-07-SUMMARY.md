---
phase: 40-release-pipeline-installer
plan: 07
subsystem: installer
tags: [inno-setup, uat]
requirements: [INST-02, INST-03, INST-04, REL-01]
key-files:
  modified: [installer/ferrisfilter.iss]
metrics:
  completed: 2026-10-18
---

# Phase 40 Plan 07: Local installer build and author UAT Summary

The locally built installer passed the author's install / reinstall / uninstall test; LICENSE.txt
stays next to ff.exe.

## Result

- Inno Setup 6 installed by the author; `installer/package.sh` built
  `dist/FerrisFilter-0.7.0-setup.exe`, the zip and `SHA256SUMS.txt` locally.
- ISCC rejected a brace comment containing `{app}`; reworded (commit fa995d5).
- Author UAT: per-user install, Start menu entry, `%APPDATA%\FerrisFilter` with the profile files,
  ff.exe working, uninstall keeping the user folder: approved. Step 8: keep `LICENSE.txt` beside
  `ff.exe`.
