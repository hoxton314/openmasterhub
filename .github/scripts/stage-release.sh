#!/usr/bin/env bash
# Stage a release: tarball of the built binaries plus the files the AUR package installs,
# and a PKGBUILD/.SRCINFO for it with the real checksum.
#
#   stage-release.sh VERSION TARGET_DIR OUT_DIR
#
# TARGET_DIR is cargo's target directory (binaries in TARGET_DIR/release). OUT_DIR receives
# openmasterhub-VERSION-x86_64.tar.gz and aur/ (PKGBUILD, .SRCINFO, openmasterhub.install).
set -euo pipefail

version=${1:?version}
target=${2:?cargo target dir}
out=${3:?output dir}
root=$(cd "$(dirname "$0")/../.." && pwd)
name="openmasterhub-${version}-x86_64"

rm -rf "${out:?}/$name" "$out/aur"
mkdir -p "$out/$name" "$out/aur"

install -m755 "$target/release/masterhubd" "$target/release/openmasterhub" "$out/$name/"
install -m644 \
  "$root/udev/70-masterhub.rules" \
  "$root/dist/masterhubd.service" \
  "$root/dist/org.openmasterhub.OpenMasterHub.desktop" \
  "$root/examples/config.toml" \
  "$root/README.md" \
  "$root/LICENSE" \
  "$out/$name/"
mkdir -p "$out/$name/icons"
for size in 32 64 128 256 512; do
  case $size in
    256) src=128x128@2x.png ;;
    512) src=icon.png ;;
    *) src=${size}x${size}.png ;;
  esac
  install -m644 "$root/app/src-tauri/icons/$src" "$out/$name/icons/${size}.png"
done
tar -C "$out" -czf "$out/$name.tar.gz" "$name"
sum=$(sha256sum "$out/$name.tar.gz" | cut -d' ' -f1)

cp "$root/aur/openmasterhub-bin/PKGBUILD" "$root/aur/openmasterhub-bin/openmasterhub.install" "$out/aur/"
sed -i \
  -e "s/^pkgver=.*/pkgver=${version}/" \
  -e "s/^pkgrel=.*/pkgrel=1/" \
  -e "s/^sha256sums=.*/sha256sums=('${sum}')/" \
  "$out/aur/PKGBUILD"
(cd "$out/aur" && makepkg --printsrcinfo > .SRCINFO)

echo "staged $out/$name.tar.gz ($sum)"
