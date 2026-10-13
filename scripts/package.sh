#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
mkdir -p dist
platform="$(uname -s)"
arch="$(uname -m)"
stage=".build/package-${platform}-${arch}"
mkdir -p "$stage/licenses" "$stage/docs"
cp LICENSE LICENSE-PENTEYE-MIT THIRD-PARTY-NOTICES.md "$stage/licenses/"
cp assets/fonts/OFL.txt "$stage/licenses/FONT-OFL.txt"
cp README.md "$stage/"
cp docs/PLATFORM-SUPPORT.md "$stage/docs/"
case "$platform" in
    Darwin)
        app="$stage/PentaKuru.app"
        mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
        cp target/release/pentakuru "$app/Contents/MacOS/pentakuru"
        chmod +x "$app/Contents/MacOS/pentakuru"
        cp assets/pentakuru-icon.png "$app/Contents/Resources/icon.png"
        cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>PentaKuru</string>
<key>CFBundleDisplayName</key><string>PentaKuru</string>
<key>CFBundleIdentifier</key><string>com.pentakuru.desktop</string>
<key>CFBundleExecutable</key><string>pentakuru</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>NSMicrophoneUsageDescription</key><string>PentaKuru uses the microphone for voice input.</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
        plutil -lint "$app/Contents/Info.plist"
        # Ad-hoc signing validates the bundle locally; this is not Apple notarization.
        codesign --force --deep --sign - "$app"
        codesign --verify --deep --strict "$app"
        hdiutil create -ov -volname PentaKuru -srcfolder "$stage" -format UDZO "dist/PentaKuru-macOS-${arch}.dmg"
        ;;
    Linux)
        cp target/release/pentakuru "$stage/pentakuru"
        chmod +x "$stage/pentakuru"
        cp assets/pentakuru-icon.png "$stage/"
        ;;
    *) echo "Unsupported packaging platform: $platform" >&2; exit 1 ;;
esac
tar -czf "dist/PentaKuru-${platform}-${arch}.tar.gz" -C "$stage" .
