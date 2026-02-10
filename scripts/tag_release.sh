#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  echo "Usage: $0 [vX.Y.Z|X.Y.Z]" >&2
  echo "Tags and pushes a release tag that MUST match Cargo.toml version." >&2
}

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is not clean. Commit or stash changes first." >&2
  exit 1
fi

VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
if [[ -z "$VERSION" ]]; then
  echo "Failed to read version from Cargo.toml" >&2
  exit 1
fi

TAG_ARG="${1:-}"
if [[ -n "$TAG_ARG" ]]; then
  case "$TAG_ARG" in
    -h|--help)
      usage
      exit 0
      ;;
  esac

  if [[ "$TAG_ARG" == v* ]]; then
    TAG="$TAG_ARG"
  else
    TAG="v$TAG_ARG"
  fi
else
  TAG="v$VERSION"
fi

if [[ "$TAG" != "v$VERSION" ]]; then
  echo "Tag/version mismatch: Cargo.toml has version=$VERSION but tag=$TAG" >&2
  echo "Update Cargo.toml (and related package manifests) or pass the matching tag." >&2
  exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Tag already exists: $TAG" >&2
  exit 1
fi

git tag -a "$TAG" -m "Release $TAG"
git push origin "$TAG"

echo "Tagged and pushed: $TAG"
