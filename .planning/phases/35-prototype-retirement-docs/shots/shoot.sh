#!/bin/bash
# usage: shoot.sh <outdir>; run from repo root with CARGO_TARGET_DIR set.
# Captures depend on scroll-animation timing, so each shot is repeated until two consecutive
# captures are byte-identical (max 12 tries) and that settled image is kept.
OUT="$1"; mkdir -p "$OUT"
export FERRISFILTER_HOME="$TEMP/ff-p35/home" USERPROFILE="$TEMP/ff-p35/up"
EXE="$CARGO_TARGET_DIR/debug/ff.exe"; F="$(pwd -W)/tests/ui/test_ff.filter"; O="$(pwd -W)/$OUT"
s(){ n=$1; shift; prev=""
  for t in $(seq 1 12); do
    rm -rf "$USERPROFILE/OneDrive"
    "$EXE" --screenshot "$O/$n.try.png" --filter "$F" "$@" 2>/dev/null
    if [ -n "$prev" ] && cmp -s "$O/$n.try.png" "$O/$n.prev.png"; then mv "$O/$n.try.png" "$O/$n.png"; rm -f "$O/$n.prev.png"; echo "$n: stable after $t tries"; return; fi
    mv "$O/$n.try.png" "$O/$n.prev.png"; prev=1
  done
  mv "$O/$n.prev.png" "$O/$n.png"; echo "$n: NOT stable after 12 tries"; }
s rules --tab rules
s rule12 --expand 12
s rule14 --expand 14
s rule16 --expand 16
s rule17 --expand 17
s rule18 --expand 18
s economy-rule --expand-economy 1
s config --tab config
s config-kite --tab config --expand-style kite
s economy --tab economy
s about --tab about
