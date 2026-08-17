#!/usr/bin/env bash
#
# Decide whether the commits since the last release warrant a new one, and if so
# compute the next version and render the changelog.
#
# Bump rules, matched against the commit subject prefix (an optional (scope) and
# "!" are allowed, e.g. "feat(parser)!: ..."):
#
#   major: -> major    feat: -> minor    fix: -> patch    anything else -> ignored
#
# If no commit in the range matches, no release is cut.
#
# PRs are squash-merged, so each squash commit's subject is the PR title and is
# what decides the bump - "feat(regex): some pull request title" counts as a
# minor. Merge commits are skipped; they carry no releasable subject here.
#
# The base version is the last vX.Y.Z tag reachable from HEAD: every commit after
# it is unreleased and feeds the decision, so a dispatch aggregates all of them
# and cuts one version. With no such tag, the first release ships the version
# already declared in Cargo.toml as-is.
#
# Outputs (stdout, plus $GITHUB_OUTPUT when set):
#   release=true|false
#   version=X.Y.Z         (only when release=true)
#   tag=vX.Y.Z            (only when release=true)
#   needs_bump=true|false (only when release=true)
# The changelog body is written to $NOTES_FILE (default: release-notes.md).
#
# needs_bump drives the two phases of a release. main takes signed commits
# through squashed pull requests only, so the version bump cannot be pushed to
# it directly; it is opened as a PR instead. needs_bump=true means Cargo.toml
# does not declare the computed version yet and that PR still has to land;
# false means it does, and the release is just a tag on the current commit.
#
# The computed version wins over whatever Cargo.toml happens to say - the last
# tag plus the commits after it are the source of truth, so a hand-edited
# Cargo.toml ahead of that gets rewritten back down.

set -euo pipefail

notes_file="${NOTES_FILE:-release-notes.md}"

emit() {
	printf '%s\n' "$1"
	if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
		printf '%s\n' "$1" >>"$GITHUB_OUTPUT"
	fi
}

# The version inside [workspace.package], ignoring any version keys in other
# tables. Every crate of the workspace inherits that one version.
package_version() {
	awk '
		/^\[/       { in_pkg = ($0 ~ /^\[workspace\.package\]/) }
		in_pkg && /^[[:space:]]*version[[:space:]]*=/ {
			gsub(/^[^=]*=[[:space:]]*"|"[[:space:]]*$/, "")
			print
			exit
		}
	' Cargo.toml
}

last_tag="$(git describe --tags --abbrev=0 --match='v[0-9]*' HEAD 2>/dev/null || true)"

if [[ -n "$last_tag" ]]; then
	range="${last_tag}..HEAD"
	base_version="${last_tag#v}"
	first_release=false
else
	range="HEAD"
	base_version="$(package_version)"
	first_release=true
fi

if [[ ! "$base_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
	echo "error: could not read a X.Y.Z base version (got '${base_version}')" >&2
	exit 1
fi
major="${BASH_REMATCH[1]}"
minor="${BASH_REMATCH[2]}"
patch="${BASH_REMATCH[3]}"

# 0 = no release, 1 = patch, 2 = minor, 3 = major.
level=0
breaking=()
features=()
fixes=()

subject_re='^(major|feat|fix)(\([^)]*\))?!?:[[:space:]]*(.*)$'

while IFS=$'\t' read -r hash subject; do
	[[ "$subject" =~ $subject_re ]] || continue
	kind="${BASH_REMATCH[1]}"
	scope="${BASH_REMATCH[2]}"
	text="${BASH_REMATCH[3]}"

	# Keep the scope visible in the entry, but drop the redundant type prefix -
	# the changelog section already says what kind of change it is.
	if [[ -n "$scope" ]]; then
		scope="${scope#(}"
		scope="${scope%)}"
		entry="- ${scope}: ${text} (${hash})"
	else
		entry="- ${text} (${hash})"
	fi

	case "$kind" in
	major)
		if ((level < 3)); then level=3; fi
		breaking+=("$entry")
		;;
	feat)
		if ((level < 2)); then level=2; fi
		features+=("$entry")
		;;
	fix)
		if ((level < 1)); then level=1; fi
		fixes+=("$entry")
		;;
	esac
done < <(git log --no-merges --format='%h%x09%s' "$range" --)

if ((level == 0)); then
	emit "release=false"
	echo "No major/feat/fix commits in ${range} - nothing to release." >&2
	exit 0
fi

if [[ "$first_release" == true ]]; then
	# Honour the version the crate already declares instead of bumping past it.
	version="${major}.${minor}.${patch}"
else
	case "$level" in
	3) version="$((major + 1)).0.0" ;;
	2) version="${major}.$((minor + 1)).0" ;;
	1) version="${major}.${minor}.$((patch + 1))" ;;
	esac
fi

section() {
	local title="$1"
	shift
	if (($# > 0)); then
		printf '## %s\n\n' "$title"
		printf '%s\n' "$@"
		printf '\n'
	fi
}

{
	section "Breaking changes" ${breaking[@]+"${breaking[@]}"}
	section "Features" ${features[@]+"${features[@]}"}
	section "Fixes" ${fixes[@]+"${fixes[@]}"}
	if [[ -n "$last_tag" ]]; then
		printf '**Full changelog**: %s/compare/%s...v%s\n' \
			"${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-}" "$last_tag" "$version"
	fi
} >"$notes_file"

if [[ "$(package_version)" == "$version" ]]; then
	needs_bump=false
else
	needs_bump=true
fi

emit "release=true"
emit "version=${version}"
emit "tag=v${version}"
emit "needs_bump=${needs_bump}"
