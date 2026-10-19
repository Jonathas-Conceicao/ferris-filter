#!/usr/bin/env bash
# Usage: release-check.sh <tag> <notes-out>
#
# Validates a release tag and writes the release notes.
#   - tag must be vX.Y.Z or vX.Y.Z-pre and equal "v" + the Cargo.toml version
#   - a version with a '-' is a pre-release: notes come from "## [Unreleased]"
#   - a final version needs a non-empty "## [X.Y.Z]" section in the CHANGELOG
# Prints "version=<v>" and "prerelease=<true|false>" on stdout (and nothing else),
# so a workflow can append stdout to $GITHUB_OUTPUT. Errors go to stderr.
# Test-only overrides: FF_CHANGELOG (default CHANGELOG.md), FF_CARGO_VERSION.
set -euo pipefail

tag="${1:?usage: release-check.sh <tag> <notes-out>}"
notes_out="${2:?usage: release-check.sh <tag> <notes-out>}"
changelog="${FF_CHANGELOG:-CHANGELOG.md}"

if ! [[ "$tag" =~ ^v([0-9]+)\.([0-9]+)\.([0-9]+)(-[0-9A-Za-z.-]+)?$ ]] || [[ "$tag" == *- ]]; then
  echo "::error::tag '$tag' is not vX.Y.Z[-pre]" >&2
  exit 1
fi

version="${tag#v}"
if [ -n "${FF_CARGO_VERSION:-}" ]; then
  cargo_version="$FF_CARGO_VERSION"
else
  cargo_version="$(cargo pkgid -p ferrisfilter | sed 's/.*[#@]//')"
fi
if [ "$version" != "$cargo_version" ]; then
  echo "::error::tag $tag != Cargo.toml version $cargo_version" >&2
  exit 1
fi

if [[ "$version" == *-* ]]; then
  key="Unreleased"
  prerelease=true
else
  key="$version"
  prerelease=false
fi

awk -v key="$key" '
  { sub(/\r$/, "") }
  /^## \[/ {
    open = 0
    if (index($0, "## [" key "]") == 1) { open = 1; next }
  }
  /^\[[^]]+\]: / { open = 0 }
  open { print }
' "$changelog" > "$notes_out"

if ! grep -q '[^[:space:]]' "$notes_out"; then
  echo "::error::CHANGELOG.md has no non-empty '## [$key]' section" >&2
  exit 1
fi

echo "version=$version"
echo "prerelease=$prerelease"
