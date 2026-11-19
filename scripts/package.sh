#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

VERSION="0.2.0"
BUILD_NUM="2"
mkdir -p dist
platform="$(uname -s)"
arch="$(uname -m)"

build_macos_icns() {
    src_png="$1"
    dest_icns="$2"
    iconset_dir="$3"
    rm -rf "$iconset_dir"
    mkdir -p "$iconset_dir"
    if command -v sips >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
        sips -z 16 16     "$src_png" --out "$iconset_dir/icon_16x16.png" >/dev/null 2>&1 || return 1
        sips -z 32 32     "$src_png" --out "$iconset_dir/icon_16x16@2x.png" >/dev/null 2>&1 || return 1
        sips -z 32 32     "$src_png" --out "$iconset_dir/icon_32x32.png" >/dev/null 2>&1 || return 1
        sips -z 64 64     "$src_png" --out "$iconset_dir/icon_32x32@2x.png" >/dev/null 2>&1 || return 1
        sips -z 128 128   "$src_png" --out "$iconset_dir/icon_128x128.png" >/dev/null 2>&1 || return 1
        sips -z 256 256   "$src_png" --out "$iconset_dir/icon_128x128@2x.png" >/dev/null 2>&1 || return 1
        sips -z 256 256   "$src_png" --out "$iconset_dir/icon_256x256.png" >/dev/null 2>&1 || return 1
        sips -z 512 512   "$src_png" --out "$iconset_dir/icon_256x256@2x.png" >/dev/null 2>&1 || return 1
        iconutil -c icns "$iconset_dir" -o "$dest_icns" >/dev/null 2>&1 || return 1
        rm -rf "$iconset_dir"
        return 0
    fi
    return 1
}

package_edition() {
    edition="$1"
    edition_label="$2"
    bundle_name="$3"
    bundle_id="$4"

    echo "=== Building PentaKuru ${edition_label} (${platform} ${arch}) ==="
    if [ "$edition" = "full" ]; then
        cargo build --release --locked
    else
        cargo build --release --locked --no-default-features --features lite
    fi

    stage=".build/package-${edition}-${platform}-${arch}"
    rm -rf "$stage"
    mkdir -p "$stage/licenses" "$stage/docs"
    cp LICENSE LICENSE-PENTEYE-MIT THIRD-PARTY-NOTICES.md "$stage/licenses/"
    cp assets/fonts/OFL.txt "$stage/licenses/FONT-OFL.txt"
    cp README.md "$stage/"
    cp docs/PLATFORM-SUPPORT.md "$stage/docs/"
    if [ "$edition" = "full" ]; then
        cp docs/OFFICE-MEDIA-INTEGRATION.md "$stage/docs/"
        cat > "$stage/EDITION.txt" <<EOF
PentaKuru Full (v${VERSION})
- Includes LibreOffice (Office Writer / Impress / Calc) integration + all core features.
- UI-TARS removed; reminders configured via AI chat and auto-paused when chatting with AI.
EOF
    else
        cat > "$stage/EDITION.txt" <<EOF
PentaKuru Lite (v${VERSION})
- Lightweight edition WITHOUT LibreOffice / Office module.
- Keeps Notes, Kanban, Search, AI Chat, Edge TTS, Spell, Sally, Penta Music, Penta Clip, and Watch Server.
- UI-TARS removed; reminders configured via AI chat and auto-paused when chatting with AI.
EOF
    fi

    case "$platform" in
        Darwin)
            app="$stage/${bundle_name}.app"
            mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
            cp target/release/pentakuru "$app/Contents/MacOS/pentakuru"
            chmod +x "$app/Contents/MacOS/pentakuru"
            cp assets/pentakuru-icon.png "$app/Contents/Resources/icon.png"
            icon_key=""
            if build_macos_icns "assets/pentakuru-icon.png" "$app/Contents/Resources/AppIcon.icns" ".build/icon-${edition}.iconset"; then
                icon_key="<key>CFBundleIconFile</key><string>AppIcon</string>"
            fi
            cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>${bundle_name}</string>
<key>CFBundleDisplayName</key><string>${bundle_name}</string>
<key>CFBundleIdentifier</key><string>${bundle_id}</string>
<key>CFBundleExecutable</key><string>pentakuru</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>${VERSION}</string>
<key>CFBundleVersion</key><string>${BUILD_NUM}</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
${icon_key}
<key>NSMicrophoneUsageDescription</key><string>PentaKuru uses the microphone for voice input and meeting translation.</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
            plutil -lint "$app/Contents/Info.plist"
            # Ad-hoc signing validates the bundle locally; this is not Apple notarization.
            codesign --force --deep --sign - "$app"
            codesign --verify --deep --strict "$app"
            tar -czf "dist/PentaKuru-${edition_label}-macOS-${arch}.tar.gz" -C "$stage" .
            ln -s /Applications "$stage/Applications"
            hdiutil create -ov -volname "${bundle_name}" -srcfolder "$stage" -format UDZO "dist/PentaKuru-${edition_label}-macOS-${arch}.dmg"
            ;;
        Linux)
            cp target/release/pentakuru "$stage/pentakuru"
            chmod +x "$stage/pentakuru"
            cp assets/pentakuru-icon.png "$stage/"
            tar -czf "dist/PentaKuru-${edition_label}-Linux-${arch}.tar.gz" -C "$stage" .
            ;;
        *)
            echo "Unsupported packaging platform: $platform" >&2
            exit 1
            ;;
    esac
}

package_edition full Full "PentaKuru Full" "com.pentakuru.desktop"
package_edition lite Lite "PentaKuru Lite" "com.pentakuru.desktop.lite"
