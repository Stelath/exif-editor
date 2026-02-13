#!/bin/bash

# Ensure we're in the project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
cd "$PROJECT_ROOT"

# Configuration
APP_NAME="Exif Editor"
VERSION=$(grep "^version =" Cargo.toml | head -1 | cut -d '"' -f 2)
DMG_NAME="ExifEditor-v${VERSION}.dmg"
BUNDLE_DIR="target/release/bundle/osx"
APP_BUNDLE="${BUNDLE_DIR}/${APP_NAME}.app"
STAGING_DIR="target/dmg_staging"

echo "🔨 Building Exif Editor in release mode..."
cargo bundle --release

if [ ! -d "$APP_BUNDLE" ]; then
    echo "❌ Error: App bundle not found at $APP_BUNDLE"
    exit 1
fi

echo "📦 Packaging into DMG..."

# Create staging directory
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"

# Copy the app bundle
cp -R "$APP_BUNDLE" "$STAGING_DIR/"

# Create a symlink to Applications folder
ln -s /Applications "$STAGING_DIR/Applications"

# Create the DMG
rm -f "$DMG_NAME"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_NAME"

# Cleanup
rm -rf "$STAGING_DIR"

echo "✅ Success! DMG created: $DMG_NAME"
