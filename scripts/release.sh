#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
DRY_RUN="${DRY_RUN:-}"

# --- Validate version argument ---
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
  echo "Usage: $0 <version>"
  echo "  version must be semver-like (e.g. 0.1.0, 0.1.0-beta.1)"
  exit 1
fi

# --- Check clean git state ---
if [ -z "$DRY_RUN" ] && [ -n "$(git status --porcelain)" ]; then
  echo "Error: Working directory is not clean. Commit or stash changes first."
  exit 1
fi

# --- Run full check pipeline ---
echo ""
echo "=== Running check pipeline ==="
pnpm run check

# --- Build for all platforms ---
echo ""
echo "=== Building for all platforms ==="
pnpm run build:all

# --- Generate release notes ---
echo ""
echo "=== Generating release notes ==="

LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

if [ -n "$LAST_TAG" ]; then
  CHANGES=$(git log "${LAST_TAG}..HEAD" --pretty=format:"- %s (%an)" 2>/dev/null)
  if [ -z "$CHANGES" ]; then
    CHANGES="No changes since ${LAST_TAG}"
  fi
else
  CHANGES=$(git log --pretty=format:"- %s (%an)" 2>/dev/null || echo "Initial release")
fi

CONTRIBUTORS=$(git log --pretty=format:"%an" "${LAST_TAG:-$(git rev-list --max-parents=0 HEAD)}..HEAD" 2>/dev/null | sort -u | sed 's/^/- /')
if [ -z "$CONTRIBUTORS" ]; then
  CONTRIBUTORS="- (none)"
fi

NOTES=$(cat <<EOF
# v${VERSION}

## What's New
${CHANGES}

## Contributors
${CONTRIBUTORS}
EOF
)

# --- Collect build artifacts ---
ARTIFACTS=""
BUNDLE_DIR="apps/web/src-tauri/target/release/bundle"
if [ -d "$BUNDLE_DIR" ]; then
  # Find dmg, deb, msi artifacts
  for ext in dmg deb msi; do
    found=$(find "$BUNDLE_DIR" -name "*.${ext}" -type f 2>/dev/null || true)
    if [ -n "$found" ]; then
      ARTIFACTS="$ARTIFACTS $found"
    fi
  done
fi

# --- Dry run: print what would happen ---
if [ -n "$DRY_RUN" ]; then
  echo ""
  echo "=== DRY RUN ==="
  echo "Would create tag: v${VERSION}"
  echo ""
  echo "Release notes:"
  echo "$NOTES"
  echo ""
  if [ -n "$ARTIFACTS" ]; then
    echo "Artifacts to attach:"
    echo "$ARTIFACTS" | tr ' ' '\n' | sed 's/^/  /'
  else
    echo "Artifacts: none found in ${BUNDLE_DIR}"
  fi
  exit 0
fi

# --- Create git tag ---
echo ""
echo "=== Creating tag v${VERSION} ==="
git tag -a "v${VERSION}" -m "Release v${VERSION}"
git push origin "v${VERSION}"

# --- Create GitHub release ---
echo ""
echo "=== Creating GitHub release ==="
# Build the gh release command
GH_ARGS="gh release create \"v${VERSION}\" --title \"v${VERSION}\" --notes \"\$NOTES\""
for artifact in $ARTIFACTS; do
  GH_ARGS="$GH_ARGS \"$artifact\""
done

eval "$GH_ARGS"

echo ""
echo "=== Release v${VERSION} complete ==="
