# editor/project-files Specification

## Purpose
Project files keep everything the editor edits in one file, to back a project up, move it to
another computer or go back to an earlier state, over the same original cartridge.

## Requirements

### Requirement: Exporting the project
The editor SHALL export the whole project, as it is being edited, to a file the user names,
offered as `exports/projects/<project>/<project>.nmbproject` (any folder may be chosen): every
level (the 32 and the new ones, with their base levels and headers), the world map and zone
map pictures, the new zones, the level each map point opens, the paths, where Mario stands on
each point, the new points, the doors, and which cartridge it belongs to. It SHALL include
what has not changed too.

#### Scenario: Export
- **WHEN** the project "Islands" is exported and the offered name is kept
- **THEN** `exports/projects/Islands/Islands.nmbproject` holds all of the above as shown in the editor, unsaved changes included, and the status bar says where it was written

#### Scenario: Export cancelled
- **WHEN** the user closes the Save dialog without a name
- **THEN** no file is written and nothing changes

### Requirement: Importing a project
The editor SHALL import a project file from any folder as a new project in `editor/projects`
(named after the file, or as the user renames it when the name is taken), with every level,
map picture, map point level, path, point and door of the file, and open it. The open project
SHALL be left as it is (asking first if it has unsaved changes).

#### Scenario: Import
- **WHEN** "friend.nmbproject", with 3 new levels and a repainted world map, is imported
- **THEN** a project "friend" appears in `editor/projects` with the file's 35 levels, world map, points, paths and doors, and the editor shows it

#### Scenario: Leave without saving
- **WHEN** a project was imported and the editor is closed without saving
- **THEN** the project that was open before is as it was before the import

### Requirement: Same cartridge only
A project file SHALL be imported only over the cartridge it was exported from; otherwise
nothing SHALL change and the status bar SHALL say the file belongs to another cartridge.

#### Scenario: Other cartridge
- **WHEN** a project exported over one cartridge is imported while a different cartridge is open
- **THEN** nothing changes and the status bar says so

### Requirement: Rejecting unusable project files
A project import SHALL change nothing when the file is not a project file or is damaged or
incomplete; the status bar SHALL say why.

#### Scenario: Damaged file
- **WHEN** a project file missing the zone maps, or a level file, is imported as a project
- **THEN** nothing changes and the status bar says the file is not a usable project file

### Requirement: Undo after a project import
A project import SHALL clear every undo history, since it is not undoable as a whole.

#### Scenario: Undo after import
- **WHEN** the user presses Undo right after importing a project
- **THEN** nothing changes

### Requirement: Older project files
Project files written before new zones existed SHALL still be imported, as projects with no
new zones.

#### Scenario: Import an older file
- **WHEN** a project file without new zones is imported over a project with two new zones
- **THEN** the project has no new zones and everything else from the file
