# editor/menu-bar Specification

## Purpose
The editor window's menu bar, like other programs have: the File menu gathers saving and the
level and project files, each entry showing its shortcut.

## Requirements

### Requirement: File menu
The editor window SHALL have a menu bar above the panels with a File menu (Spanish: Archivo)
holding, in order: Save; Export level, Import level as a new level, Replace this level from a
file; Export project, Import project. Each entry SHALL show its shortcut and run the same
action as the shortcut.

#### Scenario: Export a level from the menu
- **WHEN** the level editor is shown and the user opens File and chooses Export level
- **THEN** the Save dialog for a level file opens, as with Ctrl+E

#### Scenario: Shortcuts shown
- **WHEN** the user opens the File menu
- **THEN** each entry shows its keys at the right, for example "Export project... Ctrl+Shift+E"

### Requirement: Unavailable entries greyed out
File menu entries that cannot run where the editor is SHALL be greyed out and do nothing:
the level file entries in the world map editor, and Replace on the game's 32 levels.

#### Scenario: World map editor
- **WHEN** the world map editor is shown and the user opens the File menu
- **THEN** Save and the project entries can be chosen and the level file entries are greyed out

#### Scenario: Original level
- **WHEN** level 07 is shown and the user opens the File menu
- **THEN** Replace is greyed out, and it is available again when a new level is shown

### Requirement: No file buttons in the panels
The level and project file actions SHALL be reached from the File menu and the keyboard only;
the side panels SHALL NOT have buttons for them.

#### Scenario: Level panel
- **WHEN** the level editor is shown
- **THEN** its side panel has no export or import buttons and its palette starts where it did before this change
