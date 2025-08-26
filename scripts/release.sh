#!/usr/bin/env bash
set -eu

confirm() {
    while true; do

        read -e -p "$1 (yes/no): " yn

        case $yn in
            yes ) break;;
            no ) exit;;
        esac

    done
}

branch="main"

confirm "I have checked docs/release/checklist.md"

# Check current branch is correct

if [ "$(git rev-parse --abbrev-ref HEAD)" != "$branch" ]; then
  echo "Current branch is not $branch. Please check out that branch to make a release."
  exit 1
fi

# Check if workspace is up-to-date

echo "Querying remote to compare with local state..."

origin=$(git ls-remote origin $branch | awk '{ print $1 }')
local=$(git rev-parse $branch)

if [ "$origin" != "$local" ]; then
    echo "ERROR: local $branch's latest commit $branch differs from remote $branch's $origin."
    exit 2
fi

# Check if state is dirty
if ! git diff-index --quiet HEAD --; then
    echo "Working directory is dirty. Please commit or stash your changes."
    exit 3
fi

echo "Querying remote for old tags..."
git fetch --tags

old_ver=$(toml get -r Cargo.toml workspace.package.version)
new_ver=$(git cliff --bumped-version)

if [ "$new_ver" == "$old_ver" ]; then
    echo "No version bumping changes were made, no need for new release."
    exit 4
fi

echo -e "\n######## Confirm $old_ver -> $new_ver release ########\n"
echo -e "- Current version: $old_ver"
echo -e "- Release version: $new_ver"
echo -e "\nThe script updates the changelog; updates Cargo.toml; creates a release commit and tags it."


confirm "Are you sure?"

if [ ! -f CHANGELOG.md ]; then
    touch CHANGELOG.md
fi

git cliff --bump --unreleased --prepend CHANGELOG.md

mv Cargo.toml Cargo.toml.old
toml set Cargo.toml.old workspace.package.version "$new_ver" > Cargo.toml
rm Cargo.toml.old

echo "Please review CHANGELOG.md and add runtime compatibility changes"
echo "  - see ADR/004-using-subwasm-for-more-detailed-changelogs.md"
echo "  - see docs/release/check-runtime-compat.md"

confirm "CHANGELOG.md is properly amended"

./scripts/check.sh

git add CHANGELOG.md
git add Cargo.toml
git add Cargo.lock

git commit -m"chore: release $new_ver" --no-verify

git tag $new_ver

echo "Pushing changes..."
git push

echo "Pushing new tag..."
git push --tags

echo -e "\nAll is DONE\n"

