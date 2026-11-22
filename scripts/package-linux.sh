#!/usr/bin/env bash
# Linux packaging script: builds binary, .tar.gz archive, and .deb package
set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
VERSION="${2:-0.2.0}"
ARTIFACT_NAME="${3:-bmvp-text-linux-x64}"
SKIP_BUILD="${SKIP_BUILD:-0}"
APP_NAME="bmvp-text"
DIST_DIR="dist"

if [[ "${SKIP_BUILD}" != "1" ]]; then
    echo "==> [1/3] Building release binary for ${TARGET}..."
    cargo build --release --target "${TARGET}"
else
    echo "==> [1/3] Using prebuilt release binary for ${TARGET}..."
fi

echo "==> [2/3] Creating portable tar.gz archive..."
ARCHIVE_DIR="${DIST_DIR}/${ARTIFACT_NAME}"
rm -rf "${ARCHIVE_DIR}"
mkdir -p "${ARCHIVE_DIR}"

cp "target/${TARGET}/release/${APP_NAME}" "${ARCHIVE_DIR}/"
cp "README.md" "${ARCHIVE_DIR}/" 2>/dev/null || true
cp "LICENSE" "${ARCHIVE_DIR}/" 2>/dev/null || true
cp "assets/bmvp-text.desktop" "${ARCHIVE_DIR}/" 2>/dev/null || true
cp "assets/icon.svg" "${ARCHIVE_DIR}/" 2>/dev/null || true

tar -czf "${DIST_DIR}/${ARTIFACT_NAME}.tar.gz" -C "${DIST_DIR}" "${ARTIFACT_NAME}"
echo "==> Created ${DIST_DIR}/${ARTIFACT_NAME}.tar.gz"

echo "==> [3/3] Building Debian (.deb) package..."
DEB_DIR="${DIST_DIR}/deb_pkg"
rm -rf "${DEB_DIR}"
mkdir -p "${DEB_DIR}/DEBIAN"
mkdir -p "${DEB_DIR}/usr/bin"
mkdir -p "${DEB_DIR}/usr/share/applications"
mkdir -p "${DEB_DIR}/usr/share/icons/hicolor/scalable/apps"

cp "target/${TARGET}/release/${APP_NAME}" "${DEB_DIR}/usr/bin/"
chmod +x "${DEB_DIR}/usr/bin/${APP_NAME}"
cp "assets/bmvp-text.desktop" "${DEB_DIR}/usr/share/applications/"
cp "assets/icon.svg" "${DEB_DIR}/usr/share/icons/hicolor/scalable/apps/bmvp-text.svg"

cat > "${DEB_DIR}/DEBIAN/control" << EOF
Package: ${APP_NAME}
Version: ${VERSION}
Architecture: amd64
Maintainer: ByteMVP <dev@bytemvp.com>
Installed-Size: $(du -ks "${DEB_DIR}" | cut -f1)
Depends: libvulkan1, libwayland-client0, libxkbcommon0
Section: editors
Priority: optional
Homepage: https://github.com/bytemvp/bmvp-text
Description: Lightweight high-performance code editor inspired by Zed
 Pure Rust + GPUI 0.2, zero AI bloat, instant response.
EOF

dpkg-deb --build --root-owner-group "${DEB_DIR}" "${DIST_DIR}/${ARTIFACT_NAME}.deb"
echo "==> Created Debian package at ${DIST_DIR}/${ARTIFACT_NAME}.deb"

echo "==> Linux packaging complete!"
