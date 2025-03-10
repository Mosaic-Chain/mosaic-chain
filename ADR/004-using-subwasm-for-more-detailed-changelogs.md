---
title: 'Using subwasm for more detailed changelogs'
date: '2025-02-26'
status: approved
authors: ['vismate']
reviewers: ['wigy']
---

## Motivation

Notifying fellow developers about changes in the runtime is vital for efficient collaboration.
Currently only `git-cliff` is used to generate a changelog from our commit messages, but often times
this does not describe changes causing incompatibilities in enough detail.

[Subwasm](https://github.com/chevdor/subwasm) is a tool that can inspect runtime binaries
and help identify changes between two versions.
It would be useful to integrate subwasm into our release workflow.

## Considered Options

- Using `subwasm diff` to compare new runtime with the previous one and generate a human-readable changelog file.

`subwasm` is rather incomplete tool for now. Currently it prints diffs by debug-printing it's internal state.
This is both unstable and difficult to parse and understand for the unacquainted. This makes the output it produces
unsuitable as changelog.

- Using json output of `subwasm diff` with another tool to generate changelogs.

The json output of `subwasm diff` is also just the serialisation of it's internal state, thus it's unstable.
It would also take a considerable amount of developer effort to write a script/program that could parse the output and
reproduce it in a friendly, human readable form.

- Encouraging manual use of `subwasm diff` by developers who work on new runtime releases.

Although this adds an extra step to making a new release this simple solution still ensures
that incompatibilities are found and documented. It requires the maitainer to manually compare
runtimes (also fetch the previous one from testnet) and write the changes down in plain english.

## Decision Outcome

For now it's best to stick with the manual, but simple solution. Although asking the dev making the release to
manually compare runtimes and document the differences is cumbersome, but it's still reasonable to do so.

The list of changes for a release should appear right after the release header in `CHANGELOG.md` and before
any change description derived from commit messages. The exact format for the list of changes is not defined,
but should summarize the output of `subwasm diff` with attention to detail.

This section of the changelog should be titled "Runtime Compatibility Changes" and should contain changes
for **all** runtime variants (both solochain runtime and parachain runtime).

For example:

```markdown
...
## [1.4.2] - 2026-02-30

### Runtime Compatibility Changes

- A new event was *added* to `pallet-foo`: `ThingHappened`
- An event got *removed* from `pallet-bar`: `MacGuffin`
- Pallet index of `pallet-balances` *changed* from 4 to 6 in all runtime variants.
- ... 

### 🚀 Features

- ...
- ...

### 🐛 Bug Fixes

- ...
- ...

...
```

NOTE: The example above is not authoritative, the changes can be structured differently
under "Runtime Compatibility Changes". (Eg.: by pallet, by nature of the change, by thing that changed, ...) 

NOTE: If the runtime is fully compatible with the previous one the section "Runtime Compatibility Changes" can be omitted.
