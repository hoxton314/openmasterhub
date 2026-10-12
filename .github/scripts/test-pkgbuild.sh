#!/usr/bin/env bash
# Build the staged AUR package from the local tarball (no download) to prove the PKGBUILD
# works before anything is published. makepkg uses a source file that already exists in the
# build directory instead of fetching it, and verifies its checksum.
#
#   test-pkgbuild.sh OUT_DIR VERSION      (OUT_DIR as given to stage-release.sh)
set -euo pipefail

out=${1:?output dir}
version=${2:?version}
work=$(mktemp -d)
cp "$out/aur/PKGBUILD" "$out/aur/openmasterhub.install" "$out/openmasterhub-${version}-x86_64.tar.gz" "$work/"
(cd "$work" && makepkg --nodeps --force --noconfirm)
pkg=$(ls "$work"/openmasterhub-bin-"${version}"-1-x86_64.pkg.tar.*)
contents=$(tar -tf "$pkg" | grep -v '^\.' | sort)
echo "package contents:"
echo "$contents"
for f in usr/bin/masterhubd usr/bin/openmasterhub usr/lib/udev/rules.d/70-masterhub.rules \
         usr/lib/systemd/user/masterhubd.service usr/share/applications/org.openmasterhub.OpenMasterHub.desktop; do
  grep -qx "$f" <<<"$contents" || { echo "::error::$f missing from package"; exit 1; }
done
cp "$pkg" "$out/"
rm -rf "$work"
echo "ok: $(basename "$pkg")"
