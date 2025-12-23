#!/usr/bin/env sh
set -eu

REPO_OWNER="KarlVM12"
REPO_NAME="Dimensions"
BIN_NAME="dimensions"

usage() {
  cat <<'EOF'
Dimensions installer

Usage:
  install.sh [--version vX.Y.Z] [--dir DIR]

Options:
  --version   Tag to install (default: latest)
  --dir       Install directory (default: ~/.local/bin)

Examples:
  ./install.sh --version v0.2.5
  curl -fsSL https://raw.githubusercontent.com/KarlVM12/Dimensions/v0.2.6/install.sh | sh -s -- --version v0.2.6
EOF
}

VERSION="latest"
INSTALL_DIR="${HOME}/.local/bin"

while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --dir)
      INSTALL_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

OS="$(uname -s 2>/dev/null || echo unknown)"
ARCH="$(uname -m 2>/dev/null || echo unknown)"

case "$OS" in
  Darwin) OS="darwin" ;;
  Linux) OS="linux" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

TARGET=""
if [ "$OS" = "darwin" ] && [ "$ARCH" = "x86_64" ]; then TARGET="x86_64-apple-darwin"; fi
if [ "$OS" = "darwin" ] && [ "$ARCH" = "aarch64" ]; then TARGET="aarch64-apple-darwin"; fi
if [ "$OS" = "linux" ] && [ "$ARCH" = "x86_64" ]; then TARGET="x86_64-unknown-linux-gnu"; fi
if [ "$OS" = "linux" ] && [ "$ARCH" = "aarch64" ]; then TARGET="aarch64-unknown-linux-gnu"; fi

if [ -z "$TARGET" ]; then
  echo "Unsupported platform combination: os=$OS arch=$ARCH" >&2
  exit 1
fi

ASSET="${BIN_NAME}-${TARGET}"
URL_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases"

TMPDIR="$(mktemp -d 2>/dev/null || mktemp -d -t dimensions)"
cleanup() { rm -rf "$TMPDIR"; }
trap cleanup EXIT INT TERM

download() {
  url="$1"
  out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$out"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$out" "$url"
  else
    echo "Need curl or wget to download releases." >&2
    exit 1
  fi
}

if [ "$VERSION" = "latest" ]; then
  ASSET_URL="${URL_BASE}/latest/download/${ASSET}"
  SUM_URL="${URL_BASE}/latest/download/${ASSET}.sha256"
else
  ASSET_URL="${URL_BASE}/download/${VERSION}/${ASSET}"
  SUM_URL="${URL_BASE}/download/${VERSION}/${ASSET}.sha256"
fi

echo "Downloading ${ASSET} (${VERSION})..."
download "$ASSET_URL" "${TMPDIR}/${ASSET}"
download "$SUM_URL" "${TMPDIR}/${ASSET}.sha256"

verify_sha256() {
  file="$1"
  sumfile="$2"
  expected="$(awk '{print $1}' "$sumfile" | tr -d '\r')"
  if [ -z "$expected" ]; then
    echo "Checksum file is invalid: $sumfile" >&2
    return 1
  fi

  if command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  elif command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$file" | awk '{print $1}')"
  elif command -v openssl >/dev/null 2>&1; then
    actual="$(openssl dgst -sha256 "$file" | awk '{print $2}')"
  else
    echo "Warning: no sha256 tool found; skipping verification." >&2
    return 0
  fi

  [ "$expected" = "$actual" ]
}

if ! verify_sha256 "${TMPDIR}/${ASSET}" "${TMPDIR}/${ASSET}.sha256"; then
  echo "Checksum verification failed." >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
install_path="${INSTALL_DIR}/${BIN_NAME}"

# Atomic-ish replacement: install via temp file then move into place.
tmp_install="${install_path}.tmp.$$"
cp "${TMPDIR}/${ASSET}" "$tmp_install"
chmod +x "$tmp_install"
mv "$tmp_install" "$install_path"

echo "Installed: $install_path"
echo ""
echo "Next:"
echo "  - Ensure tmux is installed"
echo "  - Optional tmux popup binding (example):"
echo "      bind -n C-g display-popup -E -w 80% -h 80% \"dimensions\""
echo "  - Run: dimensions"
