# editor/grid-editing Specification

## Purpose

Painting a level's blocks and a map picture's tiles (world and zone maps): stamps, rectangle
selection, copy, cut, delete and paste, all undoable.

## Requirements

### Requirement: Painting with a stamp
Pencil SHALL paint the whole stamp with its top-left cell under the mouse, repeating it
seamlessly along a stroke (aligned to where the stroke started). Box SHALL fill its rectangle
with the stamp repeated from the rectangle's top-left corner. A one-cell stamp SHALL paint as
before. The hover preview SHALL show the whole stamp.

#### Scenario: Pencil with a 2x2 stamp
- **WHEN** the stamp is tiles A B / C D and the user clicks tile (10, 4) of the world map with Pencil
- **THEN** tiles (10,4)=A, (11,4)=B, (10,5)=C, (11,5)=D

#### Scenario: Box repeats the stamp
- **WHEN** the stamp is blocks A B (2 by 1) and the user drags a Box from column 0 to column 4 on one row
- **THEN** columns 0-4 become A B A B A

#### Scenario: Stamp clipped at the edge
- **WHEN** part of the stamp falls outside the level or map
- **THEN** only the cells inside are painted and nothing fails

### Requirement: Picking a stamp from the map
A right click SHALL still pick the block or tile under the mouse; a right drag SHALL pick the
rectangle dragged over as the stamp.

#### Scenario: Right drag
- **WHEN** the user right-drags over 3 by 2 tiles of a zone map
- **THEN** the stamp becomes those 3 by 2 tiles

### Requirement: Rectangle selection
A Select tool SHALL mark a rectangle on the level (Blocks mode) or the map picture (Picture
tab) by dragging; it SHALL stay outlined until another one is made, Esc is pressed or the level
or map shown changes.

#### Scenario: Select a rectangle
- **WHEN** the Select tool is chosen and the user drags from (2,3) to (5,4)
- **THEN** a 4 by 2 selection is outlined and nothing is painted

### Requirement: Copy, cut and delete
With a selection, Ctrl+C SHALL copy its cells to the clipboard; Ctrl+X SHALL copy them and fill
the selection with the background; Delete SHALL fill it with the background without copying.
The background SHALL be the block the level uses most, or the tile the map shown uses most.

#### Scenario: Cut in a level
- **WHEN** a selection is made in a level and the user presses Ctrl+X
- **THEN** the clipboard holds the selected blocks and the selection holds the level's most used block

#### Scenario: Nothing selected
- **WHEN** there is no selection and the user presses Ctrl+C
- **THEN** the clipboard does not change and the status bar says nothing is selected

### Requirement: Paste
Ctrl+V SHALL show the clipboard following the mouse (top-left under it); each left click SHALL
place it, until Esc or another tool is chosen. The clipboard SHALL remember the level or map it
was copied from, and pasting SHALL only be possible there, where the copied blocks or tiles
look the same.

#### Scenario: Paste twice
- **WHEN** the clipboard holds 3 by 3 blocks, the user presses Ctrl+V and clicks at (10,5) and then at (20,5)
- **THEN** both places hold the copied blocks

#### Scenario: Paste in another level
- **WHEN** blocks were copied in level 03 and the user shows level 07 and presses Ctrl+V
- **THEN** nothing changes and the status bar says the clipboard belongs to level 03

#### Scenario: Empty clipboard
- **WHEN** nothing was copied in that kind of view and the user presses Ctrl+V
- **THEN** nothing changes and the status bar says the clipboard is empty

### Requirement: Undo of grid edits
Each Pencil stroke, Box fill, cut, delete and paste click SHALL be one undo step of the view it
was made in (the level, or the map pictures).

#### Scenario: Undo a cut
- **WHEN** the user cuts a selection and presses Ctrl+Z
- **THEN** the selection's cells are as they were before the cut and the clipboard still holds them
