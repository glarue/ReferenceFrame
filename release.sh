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
#   bridge  platforms/mobile/rust/Cargo.toml tag: bridge-v* repo: mobile
#
#   core counts commits touching core/ (root repo); bridge counts commits
#   touching rust/ (mobile repo); app counts all other mobile-repo commits.
#
# Commit prefix → bump:
#   feat:           → minor
#   fix: / perf:    → patch
#   style: / refactor: / build: / ci: / chore: / revert: → patch
#   feat!: / feat(scope)!: / BREAKING CHANGE → major
#   docs: / test:   → no bump
#   Build-number-only commits (the sole change is pubspec.yaml's
#   `version: X.Y.Z+N` suffix, e.g. after `fastlane bump_build`) → no bump
#
# --apply refuses to run if tracked files in an affected repo have
# uncommitted changes (staged or not), so the release commit contains only
# the version bump.
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
        local breaking_re='^(feat|fix|perf|docs|style|refactor|test|build|ci|chore|revert)(\([^)]+\))?!: '
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
        local patch_re='^(style|refactor|build|ci|chore|revert)(\(.+\))?: '
        if [[ "$subject" =~ $feat_re ]]; then
            [[ "$level" != "major" ]] && level="minor"
        elif [[ "$subject" =~ $fix_re ]]; then
            [[ "$level" == "none" ]] && level="patch"
        elif [[ "$subject" =~ $patch_re ]]; then
            [[ "$level" == "none" ]] && level="patch"
        fi
        # Remaining prefixes (docs, test) → no bump
    done
    echo "$level"
}

# ── Scope definitions ────────────────────────────────────────────────────────

get_git_dir()    { if [[ "$1" == "core" ]]; then echo "$ROOT_DIR"; else echo "$MOBILE_DIR"; fi; }
get_tag_prefix() { echo "${1}-v"; }

get_version_file() {
    case "$1" in
        core)   echo "$ROOT_DIR/core/Cargo.toml" ;;
        app)    echo "$MOBILE_DIR/pubspec.yaml" ;;
        bridge) echo "$MOBILE_DIR/rust/Cargo.toml" ;;
    esac
}

# Pathspecs (relative to the scope's repo root) whose commits count toward the
# scope. Sets the global PATHSPEC array.
set_pathspec() {
    case "$1" in
        core)   PATHSPEC=("core/") ;;
        app)    PATHSPEC=("." ":!rust/") ;;  # whole mobile repo except the bridge crate
        bridge) PATHSPEC=("rust/") ;;
    esac
}

# True if a commit's only change (within the given pathspecs) is the build
# number suffix of pubspec.yaml's `version: X.Y.Z+N` line, e.g. the
# `build(app): bump build number to N` commits made after `fastlane bump_build`.
# The shipped app is unchanged, so these must not trigger a semver bump.
# Usage: is_build_number_only GIT_DIR HASH PATHSPEC...
is_build_number_only() {
    local git_dir="$1" hash="$2"
    shift 2
    local files changes
    files="$(git -C "$git_dir" diff-tree --no-commit-id --name-only -r "$hash" -- "$@" 2>/dev/null || true)"
    [[ "$files" == "pubspec.yaml" ]] || return 1
    changes="$(git -C "$git_dir" diff-tree --no-commit-id -p -U0 "$hash" -- pubspec.yaml 2>/dev/null \
        | grep -E '^[-+]' | grep -vE '^(\+\+\+|---) ' || true)"
    local re='^-version: ([0-9]+\.[0-9]+\.[0-9]+)\+[0-9]+'$'\n''\+version: ([0-9]+\.[0-9]+\.[0-9]+)\+[0-9]+$'
    [[ "$changes" =~ $re ]] && [[ "${BASH_REMATCH[1]}" == "${BASH_REMATCH[2]}" ]]
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

# ── Clean-tree check ─────────────────────────────────────────────────────────
# --apply commits whatever is staged along with the version bump, so refuse to
# run over uncommitted changes to tracked files. A core bump also commits the
# mobile Cargo.lock, so it needs both repos clean.

DIRTY_REPOS=()
for repo in "$ROOT_DIR" "$MOBILE_DIR"; do
    relevant=false
    for scope in "${SCOPES[@]}"; do
        if [[ "$scope" == "core" || "$repo" == "$MOBILE_DIR" ]]; then
            relevant=true
        fi
    done
    [[ "$relevant" == true && -d "$repo" ]] || continue
    if [[ -n "$(git -C "$repo" status --porcelain --untracked-files=no 2>/dev/null)" ]]; then
        DIRTY_REPOS+=("$repo")
    fi
done

if [[ "$APPLY" == true && ${#DIRTY_REPOS[@]} -gt 0 ]]; then
    echo "ERROR: uncommitted changes to tracked files in:" >&2
    for repo in "${DIRTY_REPOS[@]}"; do
        echo "  $repo" >&2
    done
    echo "Commit or stash them before running --apply." >&2
    exit 1
fi

# ── Main loop ────────────────────────────────────────────────────────────────

any_bump=false

for scope in "${SCOPES[@]}"; do
    git_dir="$(get_git_dir "$scope")"
    tag_prefix="$(get_tag_prefix "$scope")"
    set_pathspec "$scope"

    # Find latest tag
    latest_tag="$(git -C "$git_dir" tag -l "${tag_prefix}*" --sort=-v:refname | head -1 || true)"

    echo "[${scope}] Last tag: ${latest_tag:-(none)}"

    # Build commit range (no tag yet → all history)
    if [[ -n "$latest_tag" ]]; then
        range="${latest_tag}..HEAD"
    else
        range="HEAD"
    fi

    # Collect commits touching the scope's paths
    commits="$(git -C "$git_dir" log --oneline "$range" -- "${PATHSPEC[@]}" 2>/dev/null || true)"

    if [[ -z "$commits" ]]; then
        echo "[${scope}] No commits since ${latest_tag:-(beginning)}"
        echo ""
        continue
    fi

    echo "[${scope}] Commits since ${latest_tag:-(beginning)}:"
    counted=""
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        if is_build_number_only "$git_dir" "${line%% *}" "${PATHSPEC[@]}"; then
            echo "  ${line#* }  (ignored: build number only)"
        else
            echo "  ${line#* }"
            counted+="${line}"$'\n'
        fi
    done <<< "$commits"

    # Determine bump
    bump_level="$(printf '%s' "$counted" | determine_bump "$git_dir")"

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
        git -C "$git_dir" add "$version_file"

        # Core bump: refresh the core entry in the web bindings' Cargo.lock
        # (same repo) so it lands in the release commit instead of dirtying the
        # tree on the next WASM build. `cargo metadata` only re-resolves the
        # path dependency; it does not upgrade registry crates.
        wasm_lock="$ROOT_DIR/platforms/web/wasm_bindings/Cargo.lock"
        if [[ "$scope" == "core" && -f "$wasm_lock" ]]; then
            if ! cargo metadata --format-version 1 --manifest-path "$ROOT_DIR/platforms/web/wasm_bindings/Cargo.toml" > /dev/null; then
                echo "ERROR: failed to refresh $wasm_lock" >&2
                exit 1
            fi
            git -C "$ROOT_DIR" add "$wasm_lock"
        fi

        # If core or bridge Cargo.toml changed, update the mobile Cargo.lock
        if [[ "$scope" == "core" || "$scope" == "bridge" ]]; then
            if ! cargo update --manifest-path "$MOBILE_DIR/rust/Cargo.toml" --quiet; then
                echo "ERROR: cargo update failed for $MOBILE_DIR/rust/Cargo.toml" >&2
                echo "       The ${scope} version was bumped but the mobile Cargo.lock was not updated." >&2
                exit 1
            fi
            local_lock="$MOBILE_DIR/rust/Cargo.lock"
            if git -C "$MOBILE_DIR" diff --quiet "$local_lock" 2>/dev/null; then
                : # No lock change
            else
                git -C "$MOBILE_DIR" add "$local_lock"
                if [[ "$scope" == "core" ]]; then
                    # Core bumps happen in root repo; lock file is in mobile repo — commit separately
                    git -C "$MOBILE_DIR" commit -m "build: update Cargo.lock for ${scope} v${new_ver}"
                    echo "[${scope}] Updated mobile Cargo.lock"
                fi
                # For bridge scope, the lock file is in the same repo — include in the version commit
            fi
        fi

        git -C "$git_dir" commit -m "chore(release): ${scope} v${new_ver}"
        git -C "$git_dir" tag -a "${tag_prefix}${new_ver}" -m "${scope} v${new_ver}"
        echo "[${scope}] Created tag ${tag_prefix}${new_ver}"
        echo ""
    fi
done

if [[ "$APPLY" == false && "$any_bump" == true ]]; then
    echo "Run with --apply to execute."
    if [[ ${#DIRTY_REPOS[@]} -gt 0 ]]; then
        echo "Note: --apply will refuse until uncommitted changes are committed or stashed in:"
        for repo in "${DIRTY_REPOS[@]}"; do
            echo "  $repo"
        done
    fi
fi
