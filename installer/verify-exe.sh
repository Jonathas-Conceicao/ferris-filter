#!/usr/bin/env bash
# Usage: verify-exe.sh <exe> <version>
#
# Checks that the VERSIONINFO strings baked into a Windows exe carry the expected
# version. VERSIONINFO strings are UTF-16, so NUL bytes are stripped before the
# fixed-string search (key and value then sit next to each other).
set -euo pipefail

exe="${1:?usage: verify-exe.sh <exe> <version>}"
version="${2:?usage: verify-exe.sh <exe> <version>}"

if [ ! -f "$exe" ]; then
  echo "missing exe: $exe" >&2
  exit 1
fi

flat="$(mktemp)"
trap 'rm -f "$flat"' EXIT
tr -d '\000' < "$exe" > "$flat"

for s in \
  "ProductVersion$version" \
  "FileVersion$version" \
  "ProductNameFerrisFilter" \
  "OriginalFilenameff.exe"; do
  if ! grep -a -F -q -- "$s" "$flat"; then
    echo "missing VERSIONINFO string: $s" >&2
    exit 1
  fi
done

echo "ok: $exe carries FerrisFilter $version"
