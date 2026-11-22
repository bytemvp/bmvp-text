#!/usr/bin/env bash
# macOS packaging script: builds release binary, .app bundle, and .dmg / .tar.gz
set -euo pipefail

TARGET="${1:-aarch64-apple-darwin}"
VERSION="${2:-0.3.0}"
ARTIFACT_NAME="${3:-bmvp-text-${TARGET}}"
SKIP_BUILD="${SKIP_BUILD:-0}"
APP_NAME="bmvp-text"
DIST_DIR="dist"
APP_DIR="${DIST_DIR}/${APP_NAME}.app"

if [[ "${SKIP_BUILD}" != "1" ]]; then
    echo "==> [1/3] Building release binary for ${TARGET}..."
    cargo build --release --target "${TARGET}"
else
    echo "==> [1/3] Using prebuilt release binary for ${TARGET}..."
fi

echo "==> [2/3] Constructing macOS .app bundle..."
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS"
mkdir -p "${APP_DIR}/Contents/Resources"

cp "target/${TARGET}/release/${APP_NAME}" "${APP_DIR}/Contents/MacOS/"
chmod +x "${APP_DIR}/Contents/MacOS/${APP_NAME}"

# Write Info.plist
cat > "${APP_DIR}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.bytemvp.bmvp-text</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

echo "==> [3/3] Packaging .dmg / .tar.gz..."
if command -v create-dmg &>/dev/null; then
    create-dmg \
        --volname "${APP_NAME}" \
        --window-size 600 400 \
        --icon-size 120 \
        --app-drop-link 420 200 \
        "${DIST_DIR}/${ARTIFACT_NAME}.dmg" \
        "${APP_DIR}"
    echo "==> Created DMG at ${DIST_DIR}/${ARTIFACT_NAME}.dmg"
else
    tar -czf "${DIST_DIR}/${ARTIFACT_NAME}.tar.gz" -C "${DIST_DIR}" "${APP_NAME}.app"
    echo "==> Created archive at ${DIST_DIR}/${ARTIFACT_NAME}.tar.gz"
fi

echo "==> macOS build complete!"
