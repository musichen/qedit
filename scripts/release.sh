#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${1:-}" == -- ]]; then shift; fi
VERSION="${1:-}"
DRY_RUN="${DRY_RUN:-}"
ARTIFACT_ROOT="${QEDIT_RELEASE_ARTIFACTS:-$PROJECT_ROOT/dist/release/v${VERSION}}"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "Usage: $0 <version>"
  echo "  version must be semver-like (e.g. 0.1.0, 0.1.0-beta.1)"
  exit 1
fi

# Never publish a tag whose metadata still says 0.1.0. Updating versions belongs
# in a reviewed commit, not in a release script running on a clean checkout.
PACKAGE_VERSION=$(node -p "JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version")
TAURI_VERSION=$(node -p "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json', 'utf8')).version")
CARGO_VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' src-tauri/Cargo.toml | head -n 1)
# shellcheck disable=SC2055 # the release must abort when *any* file disagrees.
if [[ "$VERSION" != "$PACKAGE_VERSION" || "$VERSION" != "$TAURI_VERSION" || "$VERSION" != "$CARGO_VERSION" ]]; then
  echo "Error: release version $VERSION does not match project metadata" >&2
  echo "  package.json: $PACKAGE_VERSION" >&2
  echo "  src-tauri/tauri.conf.json: $TAURI_VERSION" >&2
  echo "  src-tauri/Cargo.toml: $CARGO_VERSION" >&2
  exit 1
fi

if [[ -z "$DRY_RUN" && -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo "Error: Working directory is not clean. Commit or stash changes first." >&2
  exit 1
fi

# Resolve the approved GitHub release CLI before anything is published. CI uses
# npx because gh-axi is intentionally not a project dependency; local operators
# should install gh-axi once so release publication does not fetch at runtime.
if command -v gh-axi >/dev/null 2>&1; then
  RELEASE_CLI=(gh-axi)
elif command -v npx >/dev/null 2>&1; then
  RELEASE_CLI=(npx -y gh-axi)
else
  echo "Error: gh-axi is required to create the GitHub release (install gh-axi or npx)" >&2
  exit 1
fi

echo "=== qedit v${VERSION} release plan ==="
echo "Native build targets: macos-arm64 macos-x64 windows-arm64 windows-x64 linux-arm64 linux-x64"
echo "Release root: $ARTIFACT_ROOT"
if [[ -n "$DRY_RUN" ]]; then
  echo "dry-run: no build, tag, branch push, upload, or GitHub release will be performed"
  exit 0
fi

if [[ ! -d "$ARTIFACT_ROOT" ]]; then
  echo "Error: complete CI artifact matrix is missing: $ARTIFACT_ROOT" >&2
  echo "Build each target with 'pnpm run release:build -- <target> <version>' on its native runner, then retry." >&2
  exit 1
fi
QEDIT_REQUIRE_SIGNED=1 bash "$PROJECT_ROOT/scripts/release-verify.sh" all "$VERSION" "$ARTIFACT_ROOT"

BRANCH=$(git branch --show-current)
if [[ -z "$BRANCH" || "$BRANCH" == main || "$BRANCH" == master ]]; then
  echo "Error: publish must run from a non-default release branch; current branch: ${BRANCH:-detached}" >&2
  exit 1
fi
if git ls-remote --exit-code --refs origin "refs/tags/v${VERSION}" >/dev/null 2>&1; then
  echo "Error: remote tag v${VERSION} already exists; refusing to overwrite a release" >&2
  exit 1
fi
if git show-ref --verify --quiet "refs/tags/v${VERSION}"; then
  echo "Error: local tag v${VERSION} already exists; refusing to overwrite a release" >&2
  exit 1
fi

echo "=== Generating release notes ==="
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || true)
if [[ -n "$LAST_TAG" ]]; then
  CHANGES=$(git log "${LAST_TAG}..HEAD" --pretty=format:"- %s (%an)" 2>/dev/null || true)
  [[ -n "$CHANGES" ]] || CHANGES="No changes since ${LAST_TAG}"
else
  CHANGES=$(git log --pretty=format:"- %s (%an)" 2>/dev/null || echo "Initial release")
fi

CONTRIBUTORS=$(git log --pretty=format:"%an" "${LAST_TAG:-$(git rev-list --max-parents=0 HEAD)}..HEAD" 2>/dev/null | sort -u | sed 's/^/- /' || true)
[[ -n "$CONTRIBUTORS" ]] || CONTRIBUTORS="- (none)"
NOTES=$(cat <<EOF
# v${VERSION}

## What's New
${CHANGES}

## Contributors
${CONTRIBUTORS}
EOF
)

echo ""
echo "=== Release candidate ==="
echo "Tag: v${VERSION}"
echo "Artifacts:"
find "$ARTIFACT_ROOT" -maxdepth 2 -type f \( -name 'qedit-v*' -o -name 'SHA256SUMS' -o -name 'provenance.json' \) -print | sort

echo ""
echo "=== Creating tag v${VERSION} ==="
git push origin "HEAD:refs/heads/release/v${VERSION}"
git tag -a "v${VERSION}" -m "Release v${VERSION}"
git push origin "v${VERSION}"

echo ""
echo "=== Creating GitHub release ==="
declare -a ARTIFACTS=()
while IFS= read -r -d '' artifact; do ARTIFACTS+=("$artifact"); done < <(
  find "$ARTIFACT_ROOT" -type f ! -name 'rw.*.dmg' \( -name 'qedit-v*.app.zip' -o -name 'qedit-v*.dmg' -o -name 'qedit-v*.msi' -o -name 'qedit-v*.exe' -o -name 'qedit-v*.deb' -o -name 'qedit-v*.AppImage' -o -name 'SHA256SUMS' -o -name 'provenance.json' \) -print0 | sort -z
)
if (( ${#ARTIFACTS[@]} != 14 )); then
  echo "Error: expected 12 platform artifacts plus SHA256SUMS and provenance.json, found ${#ARTIFACTS[@]}" >&2
  exit 1
fi
"${RELEASE_CLI[@]}" release create "v${VERSION}" \
  --title "v${VERSION}" \
  --notes "$NOTES" \
  --verify-tag \
  "${ARTIFACTS[@]}"

echo ""
echo "=== Release v${VERSION} complete ==="
