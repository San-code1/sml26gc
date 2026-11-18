# editor/level-files Specification

## Purpose
Level files keep a level outside the cartridge: exported from the level editor, they come
back as a new level or replace a new level whole, with the graphics they were made with.

## Requirements

### Requirement: Exporting a level
The level editor SHALL export the shown level, as it is being edited, to a file the user
names: its blocks, its objects, the level of the 32 it plays as and its header. The level
itself and the edited copy SHALL be left as they are.

#### Scenario: Export an original level
- **WHEN** level 05 is shown and the user exports it to "castle.nmblevel"
- **THEN** the file holds level 05's blocks and objects as edited, base level 05 and its header, and the status bar says where it was written

#### Scenario: Export a new level
- **WHEN** new level 34, which plays as level 12, is exported
- **THEN** the file names base level 12 and holds level 34's header, blocks and objects

#### Scenario: Export cancelled
- **WHEN** the user closes the Save dialog without a name
- **THEN** no file is written and nothing changes

### Requirement: Importing a level as a new level
The level editor SHALL import a level file as a new level after the others, playing as the
file's base level with the file's header, blocks and objects, and SHALL show it. It SHALL
refuse when there are already as many new levels as there can be.

#### Scenario: New level from a file
- **WHEN** a file exported from level 05 is imported as a new level and there are 33 levels
- **THEN** level 33 appears, plays as level 05, looks exactly like the exported level and is shown

#### Scenario: No room
- **WHEN** a level file is imported as a new level and the maximum of new levels is reached
- **THEN** no level is added and the status bar says there is no room

### Requirement: Replacing a new level from a file
The level editor SHALL replace the shown new level whole with a level file: base level,
header, blocks and objects. The replacement SHALL be undoable and redoable like any edit of
that level.

#### Scenario: Replace a new level
- **WHEN** new level 33 (plays as 02) is shown and a file exported from level 20 is imported over it
- **THEN** level 33 plays as level 20 and has the file's header, blocks and objects

#### Scenario: Undo a replacement
- **WHEN** the user presses Undo right after replacing level 33
- **THEN** level 33 has its previous base, header, blocks and objects again

### Requirement: Original levels protected
The 32 levels of the cartridge SHALL NOT be replaced by a level file: on them, the replace
action SHALL be unavailable. They SHALL remain editable as before.

#### Scenario: Replace on an original level
- **WHEN** level 07 is shown
- **THEN** the File menu's Replace entry is greyed out and its shortcut does nothing but say in the status bar that only new levels can be replaced

### Requirement: Rejecting unusable level files
An import SHALL change nothing when the file is not a level file, is damaged, names a base
level outside the 32, or does not have a whole level's blocks; the status bar SHALL say why.

#### Scenario: Wrong file
- **WHEN** the user imports a project file, or a level file cut short, as a level
- **THEN** no level is added or changed and the status bar says the file is not a usable level file

### Requirement: Imported levels saved like edited ones
An imported or replaced level SHALL be marked changed and SHALL be written into the edited
copy only when the user saves, with the same room checks as any edited level.

#### Scenario: Save after an import
- **WHEN** a level was imported and the user saves
- **THEN** it is stored with the other new levels, or the save reports what does not fit

### Requirement: Naming a new level
The level panel SHALL show a name field on a new level (32 and up); the name (up to 24
printable ASCII characters) SHALL be shown with the level in the editor, saved in the edited
copy and kept in level and project files. Level files become version 2 (with the name);
version 1 files SHALL still be read, with no name. The game's 32 levels SHALL have no name.

#### Scenario: Name a level
- **WHEN** new level 33 is shown and the user types "Ice cave" in its name field and presses Enter
- **THEN** the points panel shows "33 Ice cave" where a point opens it, and a level file exported from it has the line `name Ice cave`

#### Scenario: Old level file
- **WHEN** the user imports a level file of version 1
- **THEN** it becomes a new level with no name

### Requirement: Level file folders
Exporting a level SHALL offer the folder `exports/levels` (any folder may be chosen), and a
level imported as a new level or replacing one SHALL be part of the open project once saved.

#### Scenario: Default folder
- **WHEN** a level is exported
- **THEN** the Save dialog opens in `exports/levels`
