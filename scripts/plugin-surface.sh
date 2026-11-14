#!/usr/bin/env bash
# A pull request's change to what the plugin's users run (ai-native plan, step 5).
#
#   scripts/plugin-surface.sh BASE
#
# The marketplace serves master, so a change under integrations/claude-code/ or .claude-plugin/
# runs with every user's permissions. This says so in the job summary (informative, never blocks).
# When the pinned hashes change, it downloads SHA256SUMS from the published release named on the
# first line of checksums.txt and requires the same lines: a hash that did not come from a
# published release fails. A defence next to review, not instead of it.
#
# CI runs it on pull requests, against the base branch (.github/workflows/rust.yml, job default).
# Not part of scripts/check.sh: it needs the PR's diff and the network (D-193).
set -euo pipefail
# sort and comm must agree on the order.
export LC_ALL=C

cd "$(dirname "$0")/.."

[ "$#" -eq 1 ] || {
    echo "usage: scripts/plugin-surface.sh BASE" >&2
    exit 2
}
base=$1
checksums=integrations/claude-code/scripts/checksums.txt
repo=${GITHUB_REPOSITORY:-aquental/ripwire-broker}
summary=${GITHUB_STEP_SUMMARY:-/dev/null}

# A failing diff must fail, not read as "nothing changed".
changed=$(git diff --name-only "$base" HEAD -- integrations/claude-code .claude-plugin)
if [ -z "$changed" ]; then
    echo "no file of the plugin changed"
    exit 0
fi

echo "::warning::This PR changes what the plugin's users run: the marketplace serves master."
{
    echo "### This PR changes what the plugin's users run"
    echo
    echo "The marketplace serves \`master\`; these files run with every user's permissions:"
    echo
    while IFS= read -r file; do
        printf -- '- `%s`\n' "$file"
    done <<< "$changed"
} >> "$summary"

if ! grep -qxF "$checksums" <<< "$changed"; then
    exit 0
fi

tag=$(head -n 1 "$checksums")
if ! [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "::error::the first line of $checksums is not a version tag: '$tag'" >&2
    exit 1
fi

draft=$(gh release view "$tag" --repo "$repo" --json isDraft --jq .isDraft) || {
    echo "::error::no release $tag in $repo: the hashes in $checksums come from nowhere" >&2
    exit 1
}
if [ "$draft" != false ]; then
    echo "::error::release $tag is a draft: pin hashes only from a published release" >&2
    exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
gh release download "$tag" --repo "$repo" --pattern SHA256SUMS --output "$tmp/SHA256SUMS"
tail -n +2 "$checksums" | sort > "$tmp/pinned"
sort "$tmp/SHA256SUMS" > "$tmp/published"
if [ ! -s "$tmp/published" ]; then
    echo "::error::release $tag has an empty SHA256SUMS" >&2
    exit 1
fi
if ! cmp -s "$tmp/pinned" "$tmp/published"; then
    echo "::error::$checksums does not match the SHA256SUMS of release $tag" >&2
    echo "only in $checksums:" >&2
    comm -23 "$tmp/pinned" "$tmp/published" >&2
    echo "only in the release:" >&2
    comm -13 "$tmp/pinned" "$tmp/published" >&2
    exit 1
fi
echo "$checksums matches the SHA256SUMS of release $tag"
