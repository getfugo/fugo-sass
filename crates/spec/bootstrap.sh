#!/bin/sh
# Compiles Bootstrap with dart-sass and fugo-sass and prints the differences between the two CSS
# files, the content of crates/spec/bootstrap/<version>.diff (crates/spec/README.md).
#
# usage: crates/spec/bootstrap.sh DART_SASS FUGO_SASS BOOTSTRAP_CHECKOUT WORK_DIR
set -eu
dart_sass=$1 fugo_sass=$2 bootstrap=$3 work=$4
mkdir -p "$work"
"$dart_sass" --no-source-map "$bootstrap/scss/bootstrap.scss" > "$work/dart-sass.css" 2>/dev/null
"$fugo_sass" "$bootstrap/scss/bootstrap.scss" > "$work/fugo-sass.css" 2>/dev/null
cd "$work"
# The same diff whatever the user's git configuration.
git -c diff.mnemonicPrefix=false -c diff.noprefix=false -c diff.algorithm=myers \
  diff --no-index --no-color --no-ext-diff --src-prefix=a/ --dst-prefix=b/ \
  dart-sass.css fugo-sass.css || true
