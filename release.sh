#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"
APP_NAME="krust"

# Optional GitHub repo for upload, eg: export GITHUB_REPO="yourname/krust"
GITHUB_REPO="${GITHUB_REPO:-}"
UPLOAD="${UPLOAD:-0}"
STRIP_NIX_PATHS="${STRIP_NIX_PATHS:-1}"

VERSION="$(grep -oP '^version\s*=\s*"\K[^"]+' "$CARGO_TOML" | head -1)"
TAG="v${VERSION}"
ARCH="$(uname -m)"
PLATFORM="$(uname -s | tr '[:upper:]' '[:lower:]')"
TARBALL="${APP_NAME}-${TAG}-${PLATFORM}-${ARCH}.tar.xz"
STAGING_DIR_NAME="${APP_NAME}-${TAG}-${PLATFORM}-${ARCH}"

echo "==> Building ${APP_NAME} ${TAG} (${PLATFORM}/${ARCH})"
cd "$REPO_ROOT"
cargo build --release

BINARY="$REPO_ROOT/target/release/${APP_NAME}"
if [[ ! -f "$BINARY" ]]; then
  echo "ERROR: Binary not found at ${BINARY}"
  exit 1
fi

STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT

mkdir -p "$STAGING/$STAGING_DIR_NAME/bin"
install -m755 "$BINARY" "$STAGING/$STAGING_DIR_NAME/bin/$APP_NAME"

# Include a few useful files if present.
for path in \
  "config.example.toml" \
  "KNOWN_ISSUES.md" \
  "StackBuild.md" \
  "flake.nix"
do
  if [[ -f "$REPO_ROOT/$path" ]]; then
    install -m644 "$REPO_ROOT/$path" "$STAGING/$STAGING_DIR_NAME/"
  fi
done

if [[ "$PLATFORM" == "linux" && "$STRIP_NIX_PATHS" == "1" ]]; then
  echo "==> Stripping Nix store paths for cross-machine portability"
  # Remove build-machine-specific RPATH and set a standard glibc loader path.
  # The target system's Nix derivation (`autoPatchelfHook`) should patch these.
  run_patchelf() {
    if command -v patchelf >/dev/null 2>&1; then
      patchelf "$@"
    elif command -v nix >/dev/null 2>&1; then
      nix shell nixpkgs#patchelf -c patchelf "$@"
    else
      echo "ERROR: patchelf not found and nix is unavailable; cannot strip Nix paths"
      exit 1
    fi
  }

  run_patchelf --remove-rpath "$STAGING/$STAGING_DIR_NAME/bin/$APP_NAME"
  if [[ "$ARCH" == "x86_64" ]]; then
    run_patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 \
      "$STAGING/$STAGING_DIR_NAME/bin/$APP_NAME"
  elif [[ "$ARCH" == "aarch64" ]]; then
    run_patchelf --set-interpreter /lib/ld-linux-aarch64.so.1 \
      "$STAGING/$STAGING_DIR_NAME/bin/$APP_NAME"
  fi
fi

echo "==> Creating ${TARBALL}"
tar -cJf "$REPO_ROOT/$TARBALL" -C "$STAGING" "$STAGING_DIR_NAME"

echo "==> Created $REPO_ROOT/$TARBALL"

if [[ "$UPLOAD" == "1" ]]; then
  if [[ -z "$GITHUB_REPO" ]]; then
    echo "ERROR: UPLOAD=1 requires GITHUB_REPO=owner/repo"
    exit 1
  fi

  echo "==> Uploading to GitHub release ${TAG} (${GITHUB_REPO})"
  if gh release view "$TAG" --repo "$GITHUB_REPO" >/dev/null 2>&1; then
    gh release upload "$TAG" "$REPO_ROOT/$TARBALL" --repo "$GITHUB_REPO" --clobber
  else
    gh release create "$TAG" "$REPO_ROOT/$TARBALL" \
      --repo "$GITHUB_REPO" \
      --title "${APP_NAME} ${TAG}" \
      --notes "${APP_NAME} ${TAG}"
  fi
fi

echo "==> Done"
