# release/versions Specification

## Purpose
What a version of the programs is, when it changes and how it is published, so each published
package matches one commit and says what it brings.

## Requirements

### Requirement: A version is a number set at release time
The version SHALL be `MAJOR.MINOR.PATCH`, kept once as the workspace's `version` in `Cargo.toml`. It SHALL change only when a release is made, on `develop`, in the commit that prepares it: PATCH when the release brings only fixes, MINOR when it brings new features, MAJOR when project or save files of an earlier version no longer work. Feature branches SHALL NOT change it.

#### Scenario: A release with a new feature and a fix
- **WHEN** a release is made while "Unreleased" in the changelog lists an added feature and a fix, after version `0.1.0`
- **THEN** the version becomes `0.2.0`, set on `develop` in the commit that prepares the release

### Requirement: The changelog
`CHANGELOG.md` SHALL list, per released version (newest first, with its date) and under "Unreleased" for what is merged but not released, the changes a person using the programs notices, grouped as Added, Changed and Fixed. Each feature merged into `develop` SHALL add its entries under "Unreleased"; a release SHALL turn "Unreleased" into its version and date.

#### Scenario: Merging a feature
- **WHEN** a feature that adds a tool to the editor is merged into `develop`
- **THEN** "Unreleased" lists the tool under Added

### Requirement: Tag and GitHub release
A release SHALL merge `develop` into `main` and tag that merge `vX.Y.Z` (the version); the package built from that commit SHALL be published as the GitHub release of the tag, with the changelog's section of that version as its text. The tag SHALL be a version, never another name; a name for the release, if wanted, is its title.

#### Scenario: Publishing 0.2.0
- **WHEN** release `0.2.0` is made
- **THEN** `main` has the tag `v0.2.0` on the merge, and its GitHub release holds `sml26gc-0.2.0-windows-x64.zip` and the changelog's `0.2.0` section
