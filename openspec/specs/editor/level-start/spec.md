# editor/level-start Specification

## Purpose
Where Mario appears when a level begins: shown and placed in the level editor for the game's
levels and new levels, and played by the game from the level's own header.

## Requirements

### Requirement: Showing the level start
The level editor SHALL show, in both of its modes, where Mario appears when the shown level
begins, drawn with the game's own picture of Mario at that place, on the game's 32 levels and
on new levels. Where the picture is not available yet, a marker of Mario's size SHALL be drawn
in its place.

#### Scenario: An original level
- **WHEN** level 00 is shown
- **THEN** Mario is drawn where the game starts that level, near its left end

#### Scenario: A new level
- **WHEN** a new level copied from level 12 is shown and its start was never moved
- **THEN** Mario is drawn where level 12 starts

### Requirement: Placing the level start
The level editor SHALL let the user place the start: with "Place start" chosen, a click on
the level puts Mario there, snapped to the level's 8-pixel grid with his feet on the clicked
cell, and dragging the drawn Mario moves him. The start SHALL stay inside the level. Placing
or moving the start SHALL be undoable and redoable and SHALL mark the level changed.

#### Scenario: Place the start
- **WHEN** "Place start" is chosen on level 03 and the user clicks a floor block half way through the level
- **THEN** Mario is drawn standing on it and level 03 is marked changed

#### Scenario: Undo
- **WHEN** the user presses Undo right after moving the start
- **THEN** Mario is back where he was

### Requirement: The camera at the start
The camera a level begins with SHALL be worked out from Mario's place so that he is on the
screen, as in the game's own levels; the user SHALL NOT have to place it.

#### Scenario: Start near the right end
- **WHEN** the start is placed near the right end of a level
- **THEN** the level begins with Mario on the screen and the screen inside the level

### Requirement: Playing from the placed start
After a save, the game SHALL begin the level with Mario at the placed start and the screen
showing him, for the 32 levels and new levels; nothing else in the level SHALL change. A
level with a midway bell SHALL still restart from the bell's place after Mario rang it.

#### Scenario: Play an original level
- **WHEN** level 03's start was moved half way through, the copy is saved and level 03 is entered
- **THEN** Mario appears there, on the screen, and can play the level from there

#### Scenario: Play a new level
- **WHEN** a new level's start was moved, the copy is saved and the new level is entered from its point
- **THEN** Mario appears at its own start, not its base level's

### Requirement: The start in files and restore
The start SHALL be kept in level files and project files (it is part of the level's
header, which they already hold) and SHALL come back with them; importing a project SHALL
also give the game's 32 levels their starts from it. Restore SHALL put the start back where
the game (or, for a new level, its base level) has it.

#### Scenario: Export and import
- **WHEN** a level whose start was moved is exported and imported as a new level
- **THEN** the new level starts at the moved place

#### Scenario: Restore
- **WHEN** level 03's start was moved and the user restores level 03
- **THEN** Mario is back at the game's start of level 03
