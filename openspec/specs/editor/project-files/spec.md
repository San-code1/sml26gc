# editor/project-files Specification

## Purpose
Project files keep everything the editor edits in one file, to back a project up, move it to
another computer or go back to an earlier state, over the same original cartridge.

## Requirements

### Requirement: Exporting the project
The editor SHALL export the whole project, as it is being edited, to a file the user names:
every level (the 32 and the new ones, with their base levels and headers), the world map and
zone map pictures, the level each map point opens, the paths, where Mario stands on each
point, the new points, the doors, and which cartridge it belongs to. It SHALL include what
has not changed too.

#### Scenario: Export
- **WHEN** the user exports the project to "my world.nmbproject"
- **THEN** the file holds all of the above as shown in the editor, unsaved changes included, and the status bar says where it was written

#### Scenario: Export cancelled
- **WHEN** the user closes the Save dialog without a name
- **THEN** no file is written and nothing changes

### Requirement: Importing a project
The editor SHALL import a project file over the open project, replacing every level, map
picture, map point level, path, point and door with the file's, and SHALL mark everything
changed. Nothing SHALL be written to the edited copy until the user saves.

#### Scenario: Import
- **WHEN** a project file with 3 new levels and a repainted world map is imported over a project with 1 new level
- **THEN** the editor has the file's 35 levels, its world map, points, paths and doors, and shows the first level

#### Scenario: Leave without saving
- **WHEN** a project was imported and the editor is closed without saving
- **THEN** the edited copy is as it was before the import

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

### Requirement: World map only in projects
The world map and the zone maps SHALL be exported and imported only as part of a project.

#### Scenario: World map editor
- **WHEN** the world map editor is shown
- **THEN** the File menu offers project export and import, and no export or import of the map alone
