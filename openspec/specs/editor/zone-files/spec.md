# editor/zone-files Specification

## Purpose
Zone files keep a zone outside the cartridge: exported from the world map editor, they come
back as a new zone or replace a new zone whole, with the base zone they were made with.

## Requirements

### Requirement: Exporting a zone
The world map editor SHALL export the zone shown (one of the game's or a new one), as it is
being edited, to a file the user names: its base zone, picture, points, paths, the level
each point opens and its exits.

#### Scenario: Export the Pumpkin Zone
- **WHEN** the Pumpkin Zone is shown and the user exports it
- **THEN** the file holds the Pumpkin Zone as edited, with the Pumpkin Zone as base, and the status bar says where it was written

### Requirement: Importing a zone as a new zone
The world map editor SHALL import a zone file as a new zone, made from the file's base zone,
and SHALL show it. Levels a point opens that the project does not have SHALL open no level,
and exits that lead to points or zones the project does not have SHALL lead nowhere; the
status bar SHALL say so.

#### Scenario: New zone from a file
- **WHEN** a zone file exported from the Pumpkin Zone is imported
- **THEN** a new zone made from the Pumpkin Zone, like the exported one, is added and shown

### Requirement: Replacing a new zone from a file
The world map editor SHALL replace the new zone shown whole with a zone file, base included,
undoably. The game's zones SHALL NOT be replaced by a file; on them the action SHALL be
unavailable.

#### Scenario: Replace on a game zone
- **WHEN** the Tree Zone is shown
- **THEN** the File menu's Replace zone entry is greyed out

### Requirement: Rejecting unusable zone files
A zone import SHALL change nothing when the file is not a zone file, is damaged, names a base
that is not one of the game's zones, or has more points than its base allows; the status bar
SHALL say why.

#### Scenario: Level file as a zone
- **WHEN** the user imports a level file as a zone
- **THEN** nothing changes and the status bar says it is not a zone file
