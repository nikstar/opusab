#!/bin/sh
# Run from an extracted portable package. No sudo required.
set -eu
bundle=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix=${1:-"$HOME/.local"}
version=$("$bundle/opusab" --version | cut -d ' ' -f 2)
destination="$prefix/lib/opusab-$version"
link="$prefix/bin/opusab"
[ -d "$bundle/runtime" ] || { echo 'Run install.sh from the portable package.' >&2; exit 1; }
[ ! -e "$destination" ] || { echo "Already exists: $destination" >&2; exit 1; }
if [ -e "$link" ] || [ -L "$link" ]; then
    echo "Already exists: $link. Remove or relocate it before installing." >&2
    exit 1
fi
mkdir -p "$destination" "$prefix/bin"
cp -R "$bundle/." "$destination/"
destination=$(CDPATH= cd -- "$destination" && pwd)
ln -s "$destination/opusab" "$link"
echo "Installed $link"
echo "Add $prefix/bin to PATH if it is not already there."
