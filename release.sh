#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# release.sh — Conventional-commit-based semver bumping for ReferenceFrame
#
# Scans commits since the last scoped tag, derives semver bumps from
# conventional commit prefixes, and optionally updates version files,
# commits, and tags.
#
# Scopes:
#   core    core/Cargo.toml              tag: core-v*     repo: root
#   app     platforms/mobile/pubspec.yaml tag: app-v*      repo: mobile
#   bridge  platforms/mobile/rust/Cargo.toml tag: bridge-v* repo: root
#
# Commit prefix → bump:
#   feat:           → minor
#   fix: / perf:    → patch
#   feat!: / BREAKING CHANGE → major
#   (all others)    → no bump
#
# Usage:
#   ./release.sh                # Dry run — show what would be bumped
#   ./release.sh --apply        # Apply all bumps, commit, and tag
#   ./release.sh core           # Dry run for core only
#   ./release.sh app --apply    # Bump app only, commit and tag
#   ./release.sh -h / --help    # Show this help
#
# When to run:
#   Before a release, after conventional commits have landed. Typically:
#     1. ./release.sh            — review proposed bumps
#     2. ./release.sh --apply    — apply them
#     3. git push --follow-tags  — push commits + tags
#
#   Build numbers (pubspec.yaml +N) are NOT touched — Fastlane handles those.
# ─────────────────────────────────────────────────────────────────────────────

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
MOBILE_DIR="$ROOT_DIR/platforms/mobile"

APPLY=false
SCOPES=()

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        -h|--help)
            awk '/^# ───/{n++} n==1{sub(/^# ?/,""); print} n==2{exit}' "$0"
            exit 0
            ;;
        --apply) APPLY=true ;;
        core|app|bridge) SCOPES+=("$arg") ;;
        *) echo "Unknown argument: $arg (try --help)"; exit 1 ;;
    esac
done

# Default to all scopes
if [[ ${#SCOPES[@]} -eq 0 ]]; then
    SCOPES=(core app bridge)
fi

# ── Helpers ──────────────────────────────────────────────────────────────────

bump_version() {
    local ver="$1" level="$2"
    local major minor patch
    IFS='.' read -r major minor patch <<< "$ver"
    case "$level" in
        major) echo "$((major + 1)).0.0" ;;
        minor) echo "${major}.$((minor + 1)).0" ;;
        patch) echo "${major}.${minor}.$((patch + 1))" ;;
    esac
}

# Determine the highest bump level from a list of commits.
# Reads commit messages from stdin (one per line, format: HASH SUBJECT).
# Outputs: major, minor, patch, or none.
determine_bump() {
    local level="none"
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        local hash subject
        hash="${line%% *}"
        subject="${line#* }"

        # Check for breaking change marker in subject
        local breaking_re='^(feat|fix|perf|refactor|chore|docs|test|style|ci)!'
        if [[ "$subject" =~ $breaking_re ]]; then
            level="major"
            continue
        fi

        # Check for BREAKING CHANGE in commit body
        local body
        body="$(git -C "$1" log -1 --format=%b "$hash" 2>/dev/null || true)"
        if [[ "$body" == *"BREAKING CHANGE"* ]]; then
            level="major"
            continue
        fi

        # Map prefix to bump level
        local feat_re='^feat(\(.+\))?: '
        local fix_re='^(fix|perf)(\(.+\))?: '
        if [[ "$subject" =~ $feat_re ]]; then
            [[ "$level" != "major" ]] && level="minor"
        elif [[ "$subject" =~ $fix_re ]]; then
            [[ "$level" == "none" ]] && level="patch"
        fi
        # Other prefixes (docs, test, chore, style, refactor, ci) → no bump
    done
    echo "$level"
}

# ── Scope definitions ────────────────────────────────────────────────────────

get_git_dir()    { if [[ "$1" == "app" ]]; then echo "$MOBILE_DIR"; else echo "$ROOT_DIR"; fi; }
get_tag_prefix() { echo "${1}-v"; }

get_version_file() {
    case "$1" in
        core)   echo "$ROOT_DIR/core/Cargo.toml" ;;
        app)    echo "$MOBILE_DIR/pubspec.yaml" ;;
        bridge) echo "$MOBILE_DIR/rust/Cargo.toml" ;;
    esac
}

get_path_filter() {
    case "$1" in
        core)   echo "core/" ;;
        app)    echo "platforms/mobile/" ;;
        bridge) echo "platforms/mobile/rust/" ;;
    esac
}

read_version() {
    local scope="$1" file
    file="$(get_version_file "$scope")"
    case "$scope" in
        core|bridge)
            grep '^version = ' "$file" | head -1 | sed 's/version = "\(.*\)"/\1/'
            ;;
        app)
            grep '^version:' "$file" | head -1 | sed 's/version: \([0-9]*\.[0-9]*\.[0-9]*\).*/\1/'
            ;;
    esac
}

write_version() {
    local scope="$1" new_ver="$2" file
    file="$(get_version_file "$scope")"
    case "$scope" in
        core|bridge)
            sed -i '' "s/^version = \".*\"/version = \"${new_ver}\"/" "$file"
            ;;
        app)
            # Preserve build number: version: X.Y.Z+N → only replace X.Y.Z
            sed -i '' "s/^version: [0-9]*\.[0-9]*\.[0-9]*/version: ${new_ver}/" "$file"
            ;;
    esac
}

# ── Main loop ────────────────────────────────────────────────────────────────

any_bump=false

for scope in "${SCOPES[@]}"; do
    git_dir="$(get_git_dir "$scope")"
    tag_prefix="$(get_tag_prefix "$scope")"
    path_filter="$(get_path_filter "$scope")"

    # For app scope, path filter is relative to root but git is in mobile dir.
    # For bridge, commits are in mobile repo too but path is relative to mobile root.
    # Determine the right git dir and path for git log.
    local_git_dir="$git_dir"
    local_path_filter="$path_filter"

    if [[ "$scope" == "app" ]]; then
        # App: mobile repo, filter everything except rust/
        local_path_filter="."
    elif [[ "$scope" == "bridge" ]]; then
        # Bridge: mobile repo, filter rust/
        local_git_dir="$MOBILE_DIR"
        local_path_filter="rust/"
    fi

    # Find latest tag
    latest_tag="$(git -C "$local_git_dir" tag -l "${tag_prefix}*" --sort=-v:refname | head -1 || true)"

    echo "[${scope}] Last tag: ${latest_tag:-(none)}"

    # Build commit range
    if [[ -n "$latest_tag" ]]; then
        range="${latest_tag}..HEAD"
    else
        range="HEAD"
    fi

    # Collect commits touching the scope's paths
    if [[ "$scope" == "app" ]]; then
        # App: all commits in mobile repo, excluding rust/ directory
        if [[ -n "$latest_tag" ]]; then
            commits="$(git -C "$local_git_dir" log --oneline "$range" -- . ':!rust/' 2>/dev/null || true)"
        else
            commits="$(git -C "$local_git_dir" log --oneline -- . ':!rust/' 2>/dev/null || true)"
        fi
    elif [[ "$scope" == "core" ]]; then
        # Core: root repo, core/ directory
        if [[ -n "$latest_tag" ]]; then
            commits="$(git -C "$local_git_dir" log --oneline "$range" -- "$local_path_filter" 2>/dev/null || true)"
        else
            commits="$(git -C "$local_git_dir" log --oneline -- "$local_path_filter" 2>/dev/null || true)"
        fi
    else
        # Bridge: mobile repo, rust/ directory
        if [[ -n "$latest_tag" ]]; then
            commits="$(git -C "$local_git_dir" log --oneline "$range" -- "$local_path_filter" 2>/dev/null || true)"
        else
            commits="$(git -C "$local_git_dir" log --oneline -- "$local_path_filter" 2>/dev/null || true)"
        fi
    fi

    if [[ -z "$commits" ]]; then
        echo "[${scope}] No commits since ${latest_tag:-(beginning)}"
        echo ""
        continue
    fi

    echo "[${scope}] Commits since ${latest_tag:-(beginning)}:"
    while IFS= read -r line; do
        echo "  ${line#* }"
    done <<< "$commits"

    # Determine bump
    bump_level="$(echo "$commits" | determine_bump "$local_git_dir")"

    if [[ "$bump_level" == "none" ]]; then
        echo "[${scope}] No version-relevant commits"
        echo ""
        continue
    fi

    current_ver="$(read_version "$scope")"
    new_ver="$(bump_version "$current_ver" "$bump_level")"
    echo "[${scope}] Bump: ${bump_level} (${current_ver} → ${new_ver})"
    echo ""
    any_bump=true

    if [[ "$APPLY" == true ]]; then
        write_version "$scope" "$new_ver"
        echo "[${scope}] Updated $(get_version_file "$scope")"

        # Commit and tag
        version_file="$(get_version_file "$scope")"
        git -C "$local_git_dir" add "$version_file"
        git -C "$local_git_dir" commit -m "chore(release): ${scope} v${new_ver}"
        git -C "$local_git_dir" tag "${tag_prefix}${new_ver}"
        echo "[${scope}] Created tag ${tag_prefix}${new_ver}"
        echo ""
    fi
done

if [[ "$APPLY" == false && "$any_bump" == true ]]; then
    echo "Run with --apply to execute."
fi
