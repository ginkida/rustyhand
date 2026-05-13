#!/usr/bin/env bash
# Precompile the panel JSX modules into plain ES2018 JS using esbuild.
#
# The dashboard is single-binary: every JS file under `static/js/panel/*.js`
# is `include_str!`'d at compile time by `webchat.rs`. JSX cannot be served
# directly because we don't ship Babel — that would add ~3 MB to the binary
# and slow the page load. Instead, we precompile JSX → JS at edit time using
# this script, commit both the JSX source (under `src/`) and the compiled
# output (one level up), and the Rust build never touches Node.
#
# Run this whenever you modify a `.jsx` source in this directory.
#
# Requirements: Node + esbuild on PATH (`npm i -g esbuild` or use `npx`).
set -euo pipefail

cd "$(dirname "$0")"
OUT_DIR="$(cd .. && pwd)"

ESBUILD="${ESBUILD:-npx --yes esbuild@0.24.0}"

for f in api icons tweaks-panel pages app; do
  echo "compiling $f.jsx -> ../$f.js"
  $ESBUILD \
    --loader:.jsx=jsx \
    --jsx=transform \
    --jsx-factory=React.createElement \
    --jsx-fragment=React.Fragment \
    --target=es2018 \
    "$f.jsx" > "$OUT_DIR/$f.js"
done

echo "done."
echo "compiled bytes:"
wc -c "$OUT_DIR"/{data,api,icons,tweaks-panel,pages,app}.js
