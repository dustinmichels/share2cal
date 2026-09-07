#!/bin/bash
set -eo pipefail

export PATH="$HOME/.local/share/mise/shims:$HOME/.local/share/mise/installs/node/latest/bin:$HOME/.bun/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TAURI_DIR="$ROOT_DIR/src-tauri"

echo "=== [Share2Cal] Building Rust Backend for Xcode ==="
echo "Platform: ${PLATFORM_DISPLAY_NAME:-iOS}"
echo "Configuration: ${CONFIGURATION:-debug}"
echo "Archs: ${ARCHS:-arm64}"
echo "SDK: ${SDKROOT:-}"

# Try standard Tauri CLI xcode-script first
if bun tauri ios xcode-script -v \
    --platform "${PLATFORM_DISPLAY_NAME:-iOS}" \
    --sdk-root "${SDKROOT:-}" \
    --framework-search-paths "${FRAMEWORK_SEARCH_PATHS:-}" \
    --header-search-paths "${HEADER_SEARCH_PATHS:-}" \
    --gcc-preprocessor-definitions "${GCC_PREPROCESSOR_DEFINITIONS:-}" \
    --configuration "${CONFIGURATION:-debug}" \
    ${FORCE_COLOR:-} ${ARCHS:-arm64} 2>/dev/null; then
    echo "=== [Share2Cal] Tauri xcode-script completed successfully ==="
    exit 0
fi

echo "=== [Share2Cal] Tauri daemon not detected, running standalone build fallback ==="

# Build frontend if dist is missing
if [ ! -d "$ROOT_DIR/dist" ] || [ -z "$(ls -A "$ROOT_DIR/dist" 2>/dev/null)" ]; then
    echo "Building frontend dist..."
    (cd "$ROOT_DIR" && bun run build)
fi

# Determine target triple
IS_SIMULATOR=0
if [[ "${PLATFORM_DISPLAY_NAME:-}" =~ [Ss]imulator ]] || [[ "${SDKROOT:-}" =~ [Ss]imulator ]] || [[ "${EFFECTIVE_PLATFORM_NAME:-}" =~ [Ss]imulator ]]; then
    IS_SIMULATOR=1
fi

PRIMARY_ARCH="${ARCHS:-arm64}"
# In case ARCHS has multiple architectures, take the first one
PRIMARY_ARCH=$(echo "$PRIMARY_ARCH" | awk '{print $1}')

if [ "$IS_SIMULATOR" -eq 1 ]; then
    if [ "$PRIMARY_ARCH" = "x86_64" ]; then
        RUST_TARGET="x86_64-apple-ios"
    else
        RUST_TARGET="aarch64-apple-ios-sim"
    fi
else
    RUST_TARGET="aarch64-apple-ios"
fi

CARGO_FLAGS=()
if [ "${CONFIGURATION:-debug}" = "release" ]; then
    CARGO_FLAGS+=(--release)
    PROFILE="release"
else
    PROFILE="debug"
fi

echo "Compiling Rust library with target: $RUST_TARGET ($PROFILE)..."
(cd "$TAURI_DIR" && cargo build --target "$RUST_TARGET" "${CARGO_FLAGS[@]}")

# Ensure destination directory exists
DEST_ARCH="$PRIMARY_ARCH"
DEST_DIR="$SCRIPT_DIR/Externals/$DEST_ARCH/${CONFIGURATION:-debug}"
mkdir -p "$DEST_DIR"

SRC_LIB="$TAURI_DIR/target/$RUST_TARGET/$PROFILE/libshare2cal_lib.a"
DEST_LIB="$DEST_DIR/libapp.a"

if [ -f "$SRC_LIB" ]; then
    cp "$SRC_LIB" "$DEST_LIB"
    echo "=== [Share2Cal] Successfully installed $DEST_LIB ==="
else
    echo "ERROR: Compiled library not found at $SRC_LIB" >&2
    exit 1
fi
