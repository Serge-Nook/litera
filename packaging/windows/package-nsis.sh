#!/usr/bin/env bash
# Cross-build Литера for Windows 10/11 x64 from Linux and wrap it in an NSIS installer:
#
#   $DIST/litera-<version>-windows-x64-setup.exe
#
# Usage: packaging/windows/package-nsis.sh [--skip-build]
# Needs: rustup target x86_64-pc-windows-gnu, mingw-w64 (gcc, windres), nsis (makensis).
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/windows"
TARGET=x86_64-pc-windows-gnu

if [ "${1:-}" != "--skip-build" ]; then
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="${CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER:-x86_64-w64-mingw32-gcc}"
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C target-feature=+crt-static"
  export WORDCRAFT_REQUIRE_WINRES=1
  (cd "$ROOT" && cargo build --release --locked -p wordcraft -p wordcraft-cli --target "$TARGET")
fi
BIN="$CARGO_TARGET_DIR/$TARGET/release"
STAGE="$CARGO_TARGET_DIR/windows-nsis"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BIN/wordcraft.exe" "$STAGE/litera.exe"
cp "$BIN/wordcraft-cli.exe" "$STAGE/litera-cli.exe"
x86_64-w64-mingw32-strip "$STAGE/litera.exe" "$STAGE/litera-cli.exe" || true
for f in LICENSE-MIT LICENSE-APACHE; do [ -f "$ROOT/$f" ] && cp "$ROOT/$f" "$STAGE/"; done
# The binaries must not need MinGW runtime DLLs on a clean Windows.
if command -v x86_64-w64-mingw32-objdump >/dev/null; then
  if x86_64-w64-mingw32-objdump -p "$STAGE/litera.exe" | grep -iE 'DLL Name: (libgcc|libstdc|libwinpthread|libssp)'; then
    echo "error: litera.exe links MinGW runtime DLLs" >&2
    exit 1
  fi
fi
OUT="$(cd "$DIST" && pwd)/litera-$VERSION-windows-x64-setup.exe"
makensis -V2 -DVERSION="${VERSION%%-*}" -DBINDIR="$STAGE" -DICON="$ROOT/assets/app-icon/wordcraft.ico" -DOUTFILE="$OUT" "$HERE/litera.nsi"
echo "wrote $OUT"
