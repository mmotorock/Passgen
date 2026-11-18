#!/usr/bin/env sh
# Installs Passgen for the current user, with a launcher and icon for the app menu and dock.
#
#   packaging/linux/install.sh            build in release mode and install
#   packaging/linux/install.sh --remove   uninstall (settings and history are kept)
#
# On Wayland (e.g. GNOME), the dock shows an app's icon only if a .desktop file matching
# its app ID ("passgen") is installed; the icon embedded in the executable is not used.
set -eu

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/128x128/apps"
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"

refresh_caches() {
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR" || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q "$DATA_DIR/icons/hicolor" 2>/dev/null || true
}

if [ "${1:-}" = "--remove" ]; then
    rm -f "$BIN_DIR/passgen" "$APPS_DIR/passgen.desktop" "$ICON_DIR/passgen.png"
    refresh_caches
    echo "Passgen removed. Settings and history in ~/.config/passgen were kept."
    exit 0
fi

echo "Building Passgen in release mode..."
(cd "$REPO" && cargo build --release)

mkdir -p "$BIN_DIR" "$APPS_DIR" "$ICON_DIR"
install -m 755 "$REPO/target/release/Passgen" "$BIN_DIR/passgen"
install -m 644 "$HERE/passgen.png" "$ICON_DIR/passgen.png"
# .desktop files can't expand $HOME, so write the absolute path in.
sed "s|@EXEC@|$BIN_DIR/passgen|" "$HERE/passgen.desktop" > "$APPS_DIR/passgen.desktop"
chmod 644 "$APPS_DIR/passgen.desktop"
refresh_caches

echo "Installed:"
echo "  $BIN_DIR/passgen"
echo "  $APPS_DIR/passgen.desktop"
echo "  $ICON_DIR/passgen.png"
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Note: $BIN_DIR is not on your PATH; add it to use 'passgen' in a terminal." ;;
esac
