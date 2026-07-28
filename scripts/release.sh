#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
DRY_RUN="${DRY_RUN:-}"
BUNDLE_DIR="src-tauri/target/release/bundle"

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

echo ""
echo "=== Running check pipeline ==="
pnpm run verify

# Remove ignored bundle output before building so a failed or partial build can
# never cause an older artifact to be attached to this release.
rm -rf "$BUNDLE_DIR"
echo ""
echo "=== Building the host release bundle ==="
case "$(uname -s)" in
  Darwin) pnpm run build:mac ;;
  Linux) pnpm run build:linux ;;
  MINGW*|MSYS*|CYGWIN*) pnpm run build:win ;;
  *) echo "Unsupported release host: $(uname -s)" >&2; exit 1 ;;
esac

declare -a ARTIFACTS=()
while IFS= read -r -d '' artifact; do
  ARTIFACTS+=("$artifact")
done < <(find "$BUNDLE_DIR" -type f \( -name '*.dmg' -o -name '*.deb' -o -name '*.msi' \) -print0 2>/dev/null)
if [[ ${#ARTIFACTS[@]} -eq 0 ]]; then
  echo "Error: no release bundle was produced in $BUNDLE_DIR" >&2
  exit 1
fi

echo ""
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
printf '  %s\n' "${ARTIFACTS[@]}"
if [[ -n "$DRY_RUN" ]]; then
  echo ""
  echo "Release notes:"
  echo "$NOTES"
  exit 0
fi

echo ""
echo "=== Creating tag v${VERSION} ==="
git tag -a "v${VERSION}" -m "Release v${VERSION}"
git push origin "v${VERSION}"

echo ""
echo "=== Creating GitHub release ==="
# Arrays preserve artifact paths and release notes without eval or shell
# interpolation surprises.
npx -y gh-axi release create "v${VERSION}" \
  --title "v${VERSION}" \
  --notes "$NOTES" \
  "${ARTIFACTS[@]}"

echo ""
echo "=== Release v${VERSION} complete ==="
