#!/usr/bin/env bash
# Build masterhubd.exe and serve it with the test files to the libvirt VM network.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
target=${CARGO_TARGET_DIR:-$here/../../target}
cargo build --release --target x86_64-pc-windows-gnu -p masterhubd
out=$(mktemp -d)
cp "$target/x86_64-pc-windows-gnu/release/masterhubd.exe" "$here/config.toml" "$here/run.ps1" "$out/"
echo "serving $out on http://192.168.122.1:8765"
cd "$out" && exec python3 -m http.server 8765 --bind 192.168.122.1
