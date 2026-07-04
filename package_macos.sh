#!/usr/bin/env bash
#
# package_macos.sh — Build LMahjong and create a self-contained macOS .app bundle.
#
# The resulting .app includes all SDL2 dylibs so end-users do NOT need
# Homebrew or any other package manager. Just drag and run.
#
# Usage:
#   ./package_macos.sh
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - SDL2 libraries installed via Homebrew (build-time only):
#       brew install sdl2 sdl2_image sdl2_mixer sdl2_ttf
#   - Xcode command line tools (for install_name_tool, codesign)
#
set -euo pipefail

APP_NAME="lmahjong"
APP_DISPLAY_NAME="LMahjong"
APP_VERSION="0.1.0"
APP_IDENTIFIER="com.lmahjong.app"
APP_DESCRIPTION="A Tux-themed Mahjong solitaire game"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/target/package"
RELEASE_BIN="$SCRIPT_DIR/target/release/$APP_NAME"

ARCH="$(uname -m)"

# --- Helper functions ---

build_release() {
    echo "==> Building release binary..."

    # Ensure the linker can find Homebrew libraries
    if [ "$ARCH" = "arm64" ]; then
        export LIBRARY_PATH="/opt/homebrew/lib:${LIBRARY_PATH:-}"
    else
        export LIBRARY_PATH="/usr/local/lib:${LIBRARY_PATH:-}"
    fi

    cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
    strip "$RELEASE_BIN" 2>/dev/null || true
    echo "    Binary: $RELEASE_BIN"
}

# Resolve the actual library file from a Homebrew symlink or path
resolve_dylib() {
    local name="$1"
    local search_paths=()

    # Homebrew paths (Apple Silicon and Intel)
    if [ "$ARCH" = "arm64" ]; then
        search_paths+=("/opt/homebrew/lib")
        search_paths+=("/opt/homebrew/opt/$name/lib")
    else
        search_paths+=("/usr/local/lib")
        search_paths+=("/usr/local/opt/$name/lib")
    fi

    for dir in "${search_paths[@]}"; do
        if [ -d "$dir" ]; then
            # Find the actual dylib (not symlinks to frameworks)
            local found
            found=$(find "$dir" -maxdepth 1 -name "lib${name}*.dylib" -not -name "*-*" | head -1)
            if [ -n "$found" ]; then
                # Resolve symlinks to the actual file
                echo "$(readlink -f "$found" 2>/dev/null || echo "$found")"
                return 0
            fi
        fi
    done
    return 1
}

# Find all dylibs the binary (or another dylib) links to that come from Homebrew
# Also catches @rpath references which need resolving
find_homebrew_deps() {
    local binary="$1"
    otool -L "$binary" 2>/dev/null | tail -n +2 | awk '{print $1}' | \
        grep -E '(/opt/homebrew|/usr/local|@rpath/)' || true
}

# Resolve an @rpath reference to an actual file path in Homebrew lib dirs
resolve_rpath_dep() {
    local dep="$1"
    local basename
    basename=$(echo "$dep" | sed 's|@rpath/||')

    local search_dirs=()
    if [ "$ARCH" = "arm64" ]; then
        search_dirs+=("/opt/homebrew/lib")
    else
        search_dirs+=("/usr/local/lib")
    fi

    for dir in "${search_dirs[@]}"; do
        if [ -f "$dir/$basename" ]; then
            echo "$dir/$basename"
            return 0
        fi
    done
    return 1
}

# Recursively bundle all non-system dylib dependencies
bundle_dylibs() {
    local frameworks_dir="$1"
    local binary="$2"
    local processed_file="$frameworks_dir/.processed"

    touch "$processed_file"

    # Find Homebrew and @rpath dependencies of the binary
    local deps
    deps=$(find_homebrew_deps "$binary")

    for dep in $deps; do
        local basename
        local real_path

        if [[ "$dep" == @rpath/* ]]; then
            basename=$(echo "$dep" | sed 's|@rpath/||')
            # Resolve @rpath to actual Homebrew path
            real_path=$(resolve_rpath_dep "$dep") || true
            if [ -z "$real_path" ]; then
                echo "      Warning: Could not resolve $dep"
                continue
            fi
        else
            basename=$(basename "$dep")
            real_path=$(readlink -f "$dep" 2>/dev/null || echo "$dep")
        fi

        # Skip if already processed
        if grep -qF "$basename" "$processed_file" 2>/dev/null; then
            continue
        fi
        echo "$basename" >> "$processed_file"

        # Copy the library
        if [ -f "$real_path" ]; then
            cp "$real_path" "$frameworks_dir/$basename"
            chmod 755 "$frameworks_dir/$basename"
            echo "      Bundled: $basename"

            # Rewrite the install name in the dylib itself
            install_name_tool -id "@executable_path/../Frameworks/$basename" \
                "$frameworks_dir/$basename" 2>/dev/null || true

            # Recursively process this dylib's dependencies
            bundle_dylibs "$frameworks_dir" "$frameworks_dir/$basename"
        else
            echo "      Warning: Could not find $dep ($real_path)"
        fi
    done

    # Clean up tracking file at top level
    if [ "$binary" = "$frameworks_dir/../MacOS/$APP_NAME" ] || \
       [ "$binary" = "$(dirname "$frameworks_dir")/MacOS/$APP_NAME" ]; then
        rm -f "$processed_file"
    fi
}

# Rewrite all Homebrew and @rpath references in a binary/dylib to use @executable_path
rewrite_deps() {
    local binary="$1"
    local frameworks_dir="$2"

    local deps
    deps=$(otool -L "$binary" 2>/dev/null | tail -n +2 | awk '{print $1}' | \
        grep -E '(/opt/homebrew|/usr/local|@rpath/)' || true)

    for dep in $deps; do
        local basename
        if [[ "$dep" == @rpath/* ]]; then
            basename=$(echo "$dep" | sed 's|@rpath/||')
        else
            basename=$(basename "$dep")
        fi
        if [ -f "$frameworks_dir/$basename" ]; then
            install_name_tool -change "$dep" \
                "@executable_path/../Frameworks/$basename" \
                "$binary" 2>/dev/null || true
        fi
    done
}

build_app() {
    echo "==> Building macOS .app bundle..."

    local app_dir="$BUILD_DIR/$APP_DISPLAY_NAME.app"
    rm -rf "$app_dir"

    # Create .app bundle structure
    local contents="$app_dir/Contents"
    local macos_dir="$contents/MacOS"
    local resources_dir="$contents/Resources"
    local frameworks_dir="$contents/Frameworks"

    mkdir -p "$macos_dir"
    mkdir -p "$resources_dir/assets"
    mkdir -p "$frameworks_dir"

    # Copy binary
    cp "$RELEASE_BIN" "$macos_dir/$APP_NAME"

    # Copy assets
    cp -r "$SCRIPT_DIR/assets/"* "$resources_dir/assets/"

    # Create Info.plist
    cat > "$contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_DISPLAY_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_DISPLAY_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$APP_IDENTIFIER</string>
    <key>CFBundleVersion</key>
    <string>$APP_VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$APP_VERSION</string>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024 LMahjong. GPL-3.0-or-later.</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.board-games</string>
</dict>
</plist>
EOF

    # Create .icns icon from PNG if iconutil is available
    echo "    Creating app icon..."
    local icon_source="$SCRIPT_DIR/assets/icon.png"
    if [ -f "$icon_source" ] && command -v iconutil &>/dev/null && command -v sips &>/dev/null; then
        local iconset_dir="$BUILD_DIR/AppIcon.iconset"
        rm -rf "$iconset_dir"
        mkdir -p "$iconset_dir"

        # Generate required icon sizes
        sips -z 16 16     "$icon_source" --out "$iconset_dir/icon_16x16.png" 2>/dev/null || true
        sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_16x16@2x.png" 2>/dev/null || true
        sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_32x32.png" 2>/dev/null || true
        sips -z 64 64     "$icon_source" --out "$iconset_dir/icon_32x32@2x.png" 2>/dev/null || true
        sips -z 128 128   "$icon_source" --out "$iconset_dir/icon_128x128.png" 2>/dev/null || true
        sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_128x128@2x.png" 2>/dev/null || true
        sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_256x256.png" 2>/dev/null || true
        sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_256x256@2x.png" 2>/dev/null || true
        sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_512x512.png" 2>/dev/null || true
        sips -z 1024 1024 "$icon_source" --out "$iconset_dir/icon_512x512@2x.png" 2>/dev/null || true

        iconutil -c icns "$iconset_dir" -o "$resources_dir/AppIcon.icns" 2>/dev/null || \
            echo "      Warning: iconutil failed, app will have no icon."
        rm -rf "$iconset_dir"
    else
        echo "      Warning: Cannot create .icns (missing icon.png, iconutil, or sips)."
    fi

    # Bundle SDL2 dylibs
    echo "    Bundling SDL2 libraries..."
    bundle_dylibs "$frameworks_dir" "$macos_dir/$APP_NAME"

    # sdl2-compat loads SDL3 via dlopen at runtime — it won't appear in otool output
    # so we must explicitly bundle it.
    # sdl2-compat searches (in order): @loader_path/libSDL3.dylib, @executable_path/libSDL3.dylib
    # Due to macOS code signing/library validation, @executable_path is more reliable.
    # We place it in both MacOS/ and Frameworks/ to cover all search paths.
    echo "    Bundling SDL3 (runtime dependency of sdl2-compat)..."
    local sdl3_lib=""
    if [ "$ARCH" = "arm64" ]; then
        sdl3_lib="/opt/homebrew/lib/libSDL3.0.dylib"
    else
        sdl3_lib="/usr/local/lib/libSDL3.0.dylib"
    fi
    if [ -f "$sdl3_lib" ]; then
        local real_sdl3
        real_sdl3=$(readlink -f "$sdl3_lib" 2>/dev/null || echo "$sdl3_lib")
        # Place in Frameworks for @loader_path resolution
        cp "$real_sdl3" "$frameworks_dir/libSDL3.dylib"
        chmod 755 "$frameworks_dir/libSDL3.dylib"
        install_name_tool -id "@executable_path/../Frameworks/libSDL3.dylib" \
            "$frameworks_dir/libSDL3.dylib" 2>/dev/null || true
        # Also place in MacOS dir for @executable_path resolution (more reliable with code signing)
        cp "$real_sdl3" "$macos_dir/libSDL3.dylib"
        chmod 755 "$macos_dir/libSDL3.dylib"
        install_name_tool -id "@executable_path/libSDL3.dylib" \
            "$macos_dir/libSDL3.dylib" 2>/dev/null || true
        # Process SDL3's own dependencies
        bundle_dylibs "$frameworks_dir" "$frameworks_dir/libSDL3.dylib"
        rewrite_deps "$frameworks_dir/libSDL3.dylib" "$frameworks_dir"
        rewrite_deps "$macos_dir/libSDL3.dylib" "$frameworks_dir"
        echo "      Bundled: libSDL3.dylib (Frameworks + MacOS)"
    else
        echo "      Warning: libSDL3.0.dylib not found — sdl2-compat may fail at runtime"
    fi

    # Rewrite dylib references in the main binary
    echo "    Rewriting library paths in binary..."
    rewrite_deps "$macos_dir/$APP_NAME" "$frameworks_dir"

    # Rewrite dylib references in each bundled dylib (transitive deps)
    for dylib in "$frameworks_dir"/*.dylib; do
        [ -f "$dylib" ] && rewrite_deps "$dylib" "$frameworks_dir"
    done

    # Remove the tracking file if it still exists
    rm -f "$frameworks_dir/.processed"

    # Ad-hoc code sign (required on Apple Silicon, good practice everywhere)
    echo "    Code signing (ad-hoc)..."
    codesign --force --deep --sign - "$app_dir" 2>/dev/null || \
        echo "      Warning: codesign failed. The app may not run on Apple Silicon without signing."

    echo ""
    echo "==> Success! App bundle created:"
    echo "    $app_dir"
    echo ""
    echo "    To run:"
    echo "      open \"$app_dir\""
    echo ""
    echo "    To distribute, create a DMG:"
    echo "      hdiutil create -volname \"$APP_DISPLAY_NAME\" -srcfolder \"$app_dir\" \\"
    echo "        -ov -format UDZO \"$BUILD_DIR/${APP_DISPLAY_NAME}-${APP_VERSION}-${ARCH}.dmg\""
}

build_dmg() {
    echo "==> Creating DMG..."
    local app_dir="$BUILD_DIR/$APP_DISPLAY_NAME.app"
    local dmg_output="$BUILD_DIR/${APP_DISPLAY_NAME}-${APP_VERSION}-${ARCH}.dmg"

    if [ ! -d "$app_dir" ]; then
        echo "    Error: .app bundle not found. Run without 'dmg' argument first."
        exit 1
    fi

    # Create a temporary DMG staging area with an Applications symlink
    local dmg_staging="$BUILD_DIR/dmg-staging"
    rm -rf "$dmg_staging"
    mkdir -p "$dmg_staging"
    cp -R "$app_dir" "$dmg_staging/"
    ln -s /Applications "$dmg_staging/Applications"

    hdiutil create -volname "$APP_DISPLAY_NAME" \
        -srcfolder "$dmg_staging" \
        -ov -format UDZO \
        "$dmg_output"

    rm -rf "$dmg_staging"
    echo "    Created: $dmg_output"
}

# --- Main ---

main() {
    local target="${1:-app}"

    mkdir -p "$BUILD_DIR"
    build_release

    case "$target" in
        app)
            build_app
            ;;
        dmg)
            build_app
            build_dmg
            ;;
        *)
            echo "Usage: $0 [app|dmg]"
            echo ""
            echo "  app  - Build .app bundle (default)"
            echo "  dmg  - Build .app bundle + DMG disk image"
            exit 1
            ;;
    esac
}

main "$@"
