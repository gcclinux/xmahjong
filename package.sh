#!/usr/bin/env bash
#
# package.sh — Build LMahjong and create .deb, .rpm, and AppImage packages.
#
# Usage:
#   ./package.sh [deb|rpm|appimage|all]
#
# If no argument is given, builds all three.
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - For .deb: dpkg-deb (usually pre-installed on Debian/Ubuntu)
#   - For .rpm: rpmbuild (install: sudo dnf install rpm-build)
#   - For AppImage: wget (to fetch appimagetool if not present)
#
set -euo pipefail

APP_NAME="lmahjong"
APP_VERSION="0.1.1"
APP_DESCRIPTION="A Tux-themed Mahjong solitaire game for Linux"
APP_LICENSE="GPL-3.0-or-later"
APP_MAINTAINER="LMahjong Developer <dev@lmahjong.local>"
APP_CATEGORIES="Game;BoardGame;"
ARCH="$(uname -m)"

# Map arch names
case "$ARCH" in
    x86_64)  DEB_ARCH="amd64"; RPM_ARCH="x86_64" ;;
    aarch64) DEB_ARCH="arm64"; RPM_ARCH="aarch64" ;;
    *)       DEB_ARCH="$ARCH"; RPM_ARCH="$ARCH" ;;
esac

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/target/package"
RELEASE_BIN="$SCRIPT_DIR/target/release/$APP_NAME"

# --- Helper functions ---

build_release() {
    echo "==> Building release binary..."
    cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
    strip "$RELEASE_BIN" 2>/dev/null || true
    echo "    Binary: $RELEASE_BIN"
}

prepare_staging() {
    local staging="$1"
    rm -rf "$staging"
    mkdir -p "$staging/usr/bin"
    mkdir -p "$staging/usr/share/$APP_NAME/assets"
    mkdir -p "$staging/usr/share/applications"
    mkdir -p "$staging/usr/share/icons/hicolor/256x256/apps"

    cp "$RELEASE_BIN" "$staging/usr/bin/$APP_NAME"
    cp -r "$SCRIPT_DIR/assets/"* "$staging/usr/share/$APP_NAME/assets/"

    # Desktop file
    cat > "$staging/usr/share/applications/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Name=LMahjong
Comment=$APP_DESCRIPTION
Exec=$APP_NAME
Icon=$APP_NAME
Terminal=false
Type=Application
Categories=$APP_CATEGORIES
Keywords=mahjong;solitaire;tux;linux;puzzle;tiles;
EOF

    # Icon
    if [ -f "$SCRIPT_DIR/assets/tiles/face_05.png" ]; then
        cp "$SCRIPT_DIR/assets/tiles/face_05.png" "$staging/usr/share/icons/hicolor/256x256/apps/$APP_NAME.png"
    elif [ -f "$SCRIPT_DIR/snap/gui/icon.png" ]; then
        cp "$SCRIPT_DIR/snap/gui/icon.png" "$staging/usr/share/icons/hicolor/256x256/apps/$APP_NAME.png"
    fi
}

# --- .deb package ---

build_deb() {
    echo "==> Building .deb package..."
    local staging="$BUILD_DIR/deb-staging"
    prepare_staging "$staging"

    # DEBIAN control file
    mkdir -p "$staging/DEBIAN"
    cat > "$staging/DEBIAN/control" <<EOF
Package: $APP_NAME
Version: $APP_VERSION
Section: games
Priority: optional
Architecture: $DEB_ARCH
Depends: libsdl2-2.0-0 (>= 2.0.10), libsdl2-image-2.0-0 (>= 2.0.5), libsdl2-mixer-2.0-0 (>= 2.0.4), libsdl2-ttf-2.0-0 (>= 2.0.15)
Maintainer: $APP_MAINTAINER
Description: $APP_DESCRIPTION
 Classic tile-matching solitaire game featuring Tux penguin-themed
 graphics. Clear all 144 tiles from the board by matching pairs.
 Guaranteed solvable boards, hint system, undo, shuffle, and scoring.
EOF

    # Fix permissions
    find "$staging" -type d -exec chmod 755 {} \;
    find "$staging/usr" -type f -exec chmod 644 {} \;
    chmod 755 "$staging/usr/bin/$APP_NAME"

    local output="$BUILD_DIR/${APP_NAME}_${APP_VERSION}_${DEB_ARCH}.deb"
    dpkg-deb --build --root-owner-group "$staging" "$output"
    echo "    Created: $output"
}

# --- .rpm package ---

build_rpm() {
    echo "==> Building .rpm package..."
    local rpmbuild_dir="$BUILD_DIR/rpmbuild"
    rm -rf "$rpmbuild_dir"
    mkdir -p "$rpmbuild_dir"/{SPECS,SOURCES,BUILD,RPMS,SRPMS}

    # Create tarball for rpmbuild
    local tarball_name="${APP_NAME}-${APP_VERSION}"
    local tar_staging="$BUILD_DIR/tar-staging/$tarball_name"
    rm -rf "$BUILD_DIR/tar-staging"
    mkdir -p "$tar_staging"
    prepare_staging "$tar_staging"
    tar czf "$rpmbuild_dir/SOURCES/$tarball_name.tar.gz" -C "$BUILD_DIR/tar-staging" "$tarball_name"

    # Spec file
    cat > "$rpmbuild_dir/SPECS/$APP_NAME.spec" <<EOF
%global debug_package %{nil}

Name:           $APP_NAME
Version:        $APP_VERSION
Release:        1%{?dist}
Summary:        $APP_DESCRIPTION
License:        $APP_LICENSE
Source0:        %{name}-%{version}.tar.gz

Requires:       SDL2 >= 2.0.10
Requires:       SDL2_image >= 2.0.5
Requires:       SDL2_mixer >= 2.0.4
Requires:       SDL2_ttf >= 2.0.15

%description
Classic tile-matching solitaire game featuring Tux penguin-themed
graphics. Clear all 144 tiles from the board by matching pairs.
Guaranteed solvable boards, hint system, undo, shuffle, and scoring.

%prep
%setup -q

%install
cp -a usr %{buildroot}/usr

%files
%{_bindir}/$APP_NAME
%{_datadir}/$APP_NAME/
%{_datadir}/applications/$APP_NAME.desktop
%{_datadir}/icons/hicolor/256x256/apps/$APP_NAME.png
EOF

    rpmbuild --define "_topdir $rpmbuild_dir" --define "_dbpath $rpmbuild_dir/rpmdb" -bb "$rpmbuild_dir/SPECS/$APP_NAME.spec"
    local rpm_file=$(find "$rpmbuild_dir/RPMS" -name "*.rpm" | head -1)
    if [ -n "$rpm_file" ]; then
        cp "$rpm_file" "$BUILD_DIR/"
        echo "    Created: $BUILD_DIR/$(basename "$rpm_file")"
    else
        echo "    ERROR: rpm not found in $rpmbuild_dir/RPMS"
        return 1
    fi
}

# --- AppImage ---

build_appimage() {
    echo "==> Building AppImage..."
    local appdir="$BUILD_DIR/$APP_NAME.AppDir"
    rm -rf "$appdir"
    mkdir -p "$appdir/usr/bin"
    mkdir -p "$appdir/usr/share/$APP_NAME/assets"
    mkdir -p "$appdir/usr/lib"

    cp "$RELEASE_BIN" "$appdir/usr/bin/$APP_NAME"
    cp -r "$SCRIPT_DIR/assets/"* "$appdir/usr/share/$APP_NAME/assets/"

    # Desktop file (AppImage requires it at root of AppDir)
    cat > "$appdir/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Name=LMahjong
Comment=$APP_DESCRIPTION
Exec=$APP_NAME
Icon=$APP_NAME
Terminal=false
Type=Application
Categories=$APP_CATEGORIES
Keywords=mahjong;solitaire;tux;linux;puzzle;tiles;
EOF

    # Icon at AppDir root
    if [ -f "$SCRIPT_DIR/assets/tiles/face_05.png" ]; then
        cp "$SCRIPT_DIR/assets/tiles/face_05.png" "$appdir/$APP_NAME.png"
    elif [ -f "$SCRIPT_DIR/snap/gui/icon.png" ]; then
        cp "$SCRIPT_DIR/snap/gui/icon.png" "$appdir/$APP_NAME.png"
    else
        # Create a minimal placeholder icon
        echo "    Warning: No icon found, AppImage will have no icon."
    fi

    # Bundle SDL2 shared libraries into the AppImage
    # AppImages are self-contained — they bundle their own libs.
    echo "    Bundling SDL2 libraries..."
    for lib in libSDL2-2.0.so.0 libSDL2_image-2.0.so.0 libSDL2_mixer-2.0.so.0 libSDL2_ttf-2.0.so.0; do
        local lib_path
        lib_path=$(ldconfig -p | grep "$lib" | grep "$ARCH" | head -1 | awk '{print $NF}') || true
        if [ -z "$lib_path" ]; then
            lib_path=$(ldconfig -p | grep "$lib" | head -1 | awk '{print $NF}') || true
        fi
        if [ -n "$lib_path" ] && [ -f "$lib_path" ]; then
            cp "$lib_path" "$appdir/usr/lib/"
            echo "      Bundled: $lib"
        else
            echo "      Warning: $lib not found on system, skipping."
        fi
    done

    # Also bundle common SDL2 dependencies that may not be on all systems
    for lib in libpng16.so libfreetype.so libjpeg.so libwebp.so libtiff.so; do
        local lib_path
        lib_path=$(ldconfig -p | grep "${lib}" | grep "$ARCH" | head -1 | awk '{print $NF}') || true
        if [ -z "$lib_path" ]; then
            lib_path=$(ldconfig -p | grep "${lib}" | head -1 | awk '{print $NF}') || true
        fi
        if [ -n "$lib_path" ] && [ -f "$lib_path" ]; then
            cp "$lib_path" "$appdir/usr/lib/"
        fi
    done

    # AppRun script — sets up library path and asset path
    cat > "$appdir/AppRun" <<'APPRUN'
#!/usr/bin/env bash
SELF="$(readlink -f "$0")"
APPDIR="$(dirname "$SELF")"
export LD_LIBRARY_PATH="$APPDIR/usr/lib:${LD_LIBRARY_PATH:-}"
cd "$APPDIR/usr/share/lmahjong"
exec "$APPDIR/usr/bin/lmahjong" "$@"
APPRUN
    chmod +x "$appdir/AppRun"

    # Get appimagetool if not available
    local appimagetool="$BUILD_DIR/appimagetool"
    if [ ! -x "$appimagetool" ]; then
        echo "    Downloading appimagetool..."
        local tool_arch="$ARCH"
        wget -q -O "$appimagetool" \
            "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${tool_arch}.AppImage"
        chmod +x "$appimagetool"
    fi

    # Build the AppImage
    local output="$BUILD_DIR/${APP_NAME}-${APP_VERSION}-${ARCH}.AppImage"
    ARCH="$ARCH" "$appimagetool" "$appdir" "$output" 2>/dev/null || \
    ARCH="$ARCH" "$appimagetool" --no-appstream "$appdir" "$output"
    echo "    Created: $output"
}

# --- Main ---

main() {
    local target="${1:-all}"

    mkdir -p "$BUILD_DIR"
    build_release

    case "$target" in
        deb)
            build_deb
            ;;
        rpm)
            build_rpm
            ;;
        appimage)
            build_appimage
            ;;
        all)
            build_deb
            build_rpm
            build_appimage
            ;;
        *)
            echo "Usage: $0 [deb|rpm|appimage|all]"
            exit 1
            ;;
    esac

    echo ""
    echo "==> Packages in: $BUILD_DIR/"
    ls -lh "$BUILD_DIR"/*.deb "$BUILD_DIR"/*.rpm "$BUILD_DIR"/*.AppImage 2>/dev/null || true
}

main "$@"
