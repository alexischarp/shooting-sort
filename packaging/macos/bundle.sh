#!/bin/sh
# Builds "Shooting Sort.app" and a .dmg from a compiled binary.
# Usage: packaging/macos/bundle.sh <binary> <version> <output-dir>
set -eu

binary=$1
version=$2
out=$3
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

app="$out/Shooting Sort.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/shooting-sort"
cp "$root/assets/icon.icns" "$app/Contents/Resources/icon.icns"
sed "s/@VERSION@/$version/g" "$here/Info.plist" > "$app/Contents/Info.plist"

# Ad-hoc signature: not a developer identity, but Apple Silicon refuses to run unsigned code.
codesign --force --deep --sign - "$app"

# Disk image with a link to /Applications, so the app can be dragged into it.
staging="$out/dmg"
rm -rf "$staging"
mkdir -p "$staging"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
hdiutil create -volname "Shooting Sort" -srcfolder "$staging" -ov -format UDZO "$out/Shooting-Sort-$version-macos.dmg"
rm -rf "$staging"
