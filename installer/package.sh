#!/usr/bin/env bash
# Usage: installer/package.sh <version>
#
# Builds dist/: the portable zip, the setup.exe (Inno Setup) and SHA256SUMS.txt.
# Run from anywhere after `cargo build --release --bin ff`; CI and local runs use this script.
#
# Environment:
#   FF_EXE=<path>          the ff.exe to package (default target/release/ff.exe)
#   FF_SKIP_INSTALLER=1    local only: skip ISCC; the result is not a releasable set
set -euo pipefail

ver="${1:?usage: package.sh <version>}"
num="${ver%%-*}"   # 0.7.0-rc.1 -> 0.7.0 (the version resource must be numeric)
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist="$root/dist"
stage="$dist/stage"
exe="${FF_EXE:-$root/target/release/ff.exe}"

if [ ! -f "$exe" ]; then
  echo "missing $exe: build it first (cargo build --release --bin ff)" >&2
  exit 1
fi

find_iscc() {
  local c local_app
  local_app=""
  if [ -n "${LOCALAPPDATA:-}" ]; then
    local_app="$(cygpath -u "$LOCALAPPDATA")/Programs/Inno Setup 6/ISCC.exe"
  fi
  for c in "$(command -v iscc.exe || true)" \
           "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" \
           "$local_app"; do
    if [ -n "$c" ] && [ -f "$c" ]; then
      echo "$c"
      return 0
    fi
  done
  echo "ISCC.exe not found: install Inno Setup 6 (winget install --id JRSoftware.InnoSetup -e)" >&2
  return 1
}

# Resolve ISCC before touching dist/, so a missing tool fails fast.
iscc=""
if [ "${FF_SKIP_INSTALLER:-}" != "1" ]; then
  iscc="$(find_iscc)"
fi

rm -rf "$dist"
mkdir -p "$stage"

cp "$exe" "$stage/ff.exe"
cp "$root/LICENSE" "$stage/LICENSE.txt"
cp "$root/installer/portable-readme.txt" "$stage/README.txt"

zip="FerrisFilter-$ver-windows-x64.zip"
# The full path matters: Git Bash's GNU tar cannot write zip archives.
/c/Windows/System32/tar.exe -a -c -f "$dist/$zip" -C "$stage" ff.exe LICENSE.txt README.txt

setup=""
if [ -n "$iscc" ]; then
  MSYS2_ARG_CONV_EXCL='*' "$iscc" /Q "/DAppVersion=$ver" "/DAppNumVersion=$num" \
    "/DAppExe=$(cygpath -w "$exe")" "$(cygpath -w "$root/installer/ferrisfilter.iss")"
  setup="FerrisFilter-$ver-setup.exe"
  if [ ! -f "$dist/$setup" ]; then
    echo "ISCC produced no $dist/$setup" >&2
    exit 1
  fi
else
  echo "WARNING: installer skipped (FF_SKIP_INSTALLER=1): this is not a releasable set" >&2
fi

rm -rf "$stage"

(
  cd "$dist"
  for f in $setup "$zip"; do
    printf '%s  %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$f"
  done
) > "$dist/SHA256SUMS.txt"

ls -l "$dist"
