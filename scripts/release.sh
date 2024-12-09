#!/usr/bin/env bash
set -eu

develop="develop"

# Check current branch is develop

if [ "$(git rev-parse --abbrev-ref HEAD)" != "$develop" ]; then
  echo "Current branch is not $develop. Please check out that branch to make a release."
  exit 1
fi

# Check if workspace is up-to-date

echo "Querying remote to compare with local state..."

origin=$(git ls-remote origin $develop | awk '{ print $1 }')
local=$(git rev-parse $develop)

if [ "$origin" != "$local" ]; then
    echo "ERROR: local $develop's latest commit $local differs from remote $develop's $origin."
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

while true; do

    read -e -p "Are you sure? (yes/no) " yn

    case $yn in
        yes ) break;;
        no ) exit;;
    esac

done

./scripts/check.sh

git cliff --bump > CHANGELOG.md

mv Cargo.toml Cargo.toml.old
toml set Cargo.toml.old workspace.package.version "$new_ver" > Cargo.toml
rm Cargo.toml.old

git add CHANGELOG.md
git add Cargo.toml

git commit -m"chore: release $new_ver"

git tag $new_ver

echo "Pushing changes..."
git push

echo "Pushing new tag..."
git push --tags

echo -e "\nAll is DONE\n"
echo -e "- Create a merge request in GitLab from $develop to testnet/devnet/mainnet."
echo -e "- Merge it."

