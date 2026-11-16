# editor/blank-levels Specification

## Purpose
Making an empty new level from one of the game's levels, chosen while seeing only the block
palette and the object palette it brings, so a level can be built from nothing with tiles and
objects that belong together.

## Requirements

### Requirement: Choosing the level a blank level comes from
New blank level SHALL open a chooser of the game's 32 levels. While a level is chosen in it,
the editor SHALL show only that level's block palette and its object palette, drawn as the game
draws them in that level; objects whose picture is not captured yet SHALL show as numbered
boxes until it is. Esc or Cancel SHALL close it without making a level.

#### Scenario: Looking at two levels
- **WHEN** New blank level is chosen and level 01, then level 18, is picked in the chooser
- **THEN** the palettes shown are level 01's blocks and objects, then level 18's

#### Scenario: Cancel
- **WHEN** the chooser is open and Esc is pressed
- **THEN** no level is made and the level shown before is shown again

### Requirement: What a blank level is made of
Making a blank level from game level L SHALL add a new level that plays as L (its tiles,
objects, music, time, shades and settings) with an empty map: L's sky block everywhere but a
floor along the bottom made of the block Mario stands on at L's start, Mario's start on that
floor near the left, and no objects. It SHALL be undoable, saved and filed like any new level.

#### Scenario: A blank level from level 01
- **WHEN** a blank level is made from level 01
- **THEN** a new level is shown with level 01's palettes, an empty sky, a floor and Mario's start, and the status bar says it plays as level 01

#### Scenario: Played
- **WHEN** a blank level is saved and played from the editor
- **THEN** Mario starts on its floor and the level looks as level 01 does

### Requirement: Where a blank level can come from
The chooser SHALL say, for the level picked, what comes with it besides its pictures: its
gravity, camera margin and animated tiles (which can be changed afterwards), and that level 24
is Wario's final room.

#### Scenario: A Space Zone level
- **WHEN** level 18 is picked in the chooser
- **THEN** the chooser says the new level will have space gravity, which its settings can change
