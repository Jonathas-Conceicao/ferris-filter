#!/usr/bin/env bash
# Test cases for installer/release-check.sh. Run from the repo root:
#   bash installer/release-check.test.sh
set -uo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0

pass() { echo "PASS $1"; }
fail() { echo "FAIL $1${2:+: $2}"; fails=$((fails + 1)); }

cat > "$tmp/cl.md" <<'CL'
# Changelog

## [Unreleased]

Pre-release note line.

- item one

## [0.7.0-rc.1] - 2026-10-30

rc section body

## [0.7.0] - 2026-11-01

Final body line.

## [0.6.0] - 2026-10-17

Old body.

[Unreleased]: https://example.invalid/compare/v0.6...HEAD
[0.6.0]: https://example.invalid/compare/v0.5...v0.6
CL
sed 's/$/\r/' "$tmp/cl.md" > "$tmp/cl-crlf.md"

# A blank-only section, and a last section that runs straight into the link block.
cat > "$tmp/cl2.md" <<'CL'
## [Unreleased]

## [0.8.0] - 2026-12-01



## [0.7.0] - 2026-11-01

Last body.
[Unreleased]: https://example.invalid/x
[0.7.0]: https://example.invalid/y
CL

# run <changelog> <cargo-version> <tag>; sets rc; stdout/stderr in $tmp/out, $tmp/err; notes in $tmp/notes.md
run() {
  rm -f "$tmp/notes.md"
  FF_CHANGELOG="$1" FF_CARGO_VERSION="$2" bash installer/release-check.sh "$3" "$tmp/notes.md" \
    > "$tmp/out" 2> "$tmp/err"
  rc=$?
}

expect_fail() { # name changelog cargo tag stderr-substring
  run "$2" "$3" "$4"
  if [ "$rc" -ne 0 ] && grep -q -F -- "$5" "$tmp/err"; then pass "$1"; else fail "$1" "rc=$rc err=$(cat "$tmp/err")"; fi
}

expect_ok() { # name changelog cargo tag version prerelease expected-notes-file
  run "$2" "$3" "$4"
  printf 'version=%s\nprerelease=%s\n' "$5" "$6" > "$tmp/want-out"
  if [ "$rc" -eq 0 ] && diff -q "$tmp/out" "$tmp/want-out" > /dev/null \
     && diff -q "$tmp/notes.md" "$7" > /dev/null \
     && [ "$(tr -cd '\r' < "$tmp/notes.md" | wc -c)" -eq 0 ]; then
    pass "$1"
  else
    fail "$1" "rc=$rc out=$(cat "$tmp/out") err=$(cat "$tmp/err")"
  fi
}

printf '\nPre-release note line.\n\n- item one\n\n' > "$tmp/want-unreleased"
printf '\nFinal body line.\n\n' > "$tmp/want-final"
printf '\nLast body.\n' > "$tmp/want-last"

expect_fail "tag-differs-from-cargo" "$tmp/cl.md" 0.6.0 v9.9.9 "!= Cargo.toml version"
for bad in 0.7.0 v0.7 v0.7.0.1 v0.7.0-; do
  expect_fail "bad-tag-$bad" "$tmp/cl.md" 0.7.0 "$bad" "is not vX.Y.Z"
done
expect_ok "prerelease-uses-unreleased" "$tmp/cl.md" 0.7.0-rc.1 v0.7.0-rc.1 0.7.0-rc.1 true "$tmp/want-unreleased"
expect_ok "final-uses-own-section" "$tmp/cl.md" 0.7.0 v0.7.0 0.7.0 false "$tmp/want-final"
expect_fail "final-missing-section" "$tmp/cl.md" 0.9.0 v0.9.0 "no non-empty"
expect_fail "final-blank-section" "$tmp/cl2.md" 0.8.0 v0.8.0 "no non-empty"
expect_ok "crlf-prerelease" "$tmp/cl-crlf.md" 0.7.0-rc.1 v0.7.0-rc.1 0.7.0-rc.1 true "$tmp/want-unreleased"
expect_ok "crlf-final" "$tmp/cl-crlf.md" 0.7.0 v0.7.0 0.7.0 false "$tmp/want-final"
expect_ok "stops-at-link-references" "$tmp/cl2.md" 0.7.0 v0.7.0 0.7.0 false "$tmp/want-last"

# Real CHANGELOG: the newest released section extracts to non-empty notes. [Unreleased] is
# not used here, since it is legitimately empty between milestones.
latest="$(sed -n 's/\r$//; s/^## \[\([0-9][0-9.]*\)\] - .*/\1/p' CHANGELOG.md | head -n1)"
if [ -z "$latest" ]; then
  fail "real-changelog-final" "no released section in CHANGELOG.md"
else
  run CHANGELOG.md "$latest" "v$latest"
  if [ "$rc" -eq 0 ] && grep -q '[^[:space:]]' "$tmp/notes.md"; then pass "real-changelog-final"
  else fail "real-changelog-final" "$(cat "$tmp/err")"; fi
fi

if [ "$fails" -ne 0 ]; then echo "$fails case(s) failed"; exit 1; fi
echo "all cases passed"
