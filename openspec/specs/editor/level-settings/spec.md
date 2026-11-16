# editor/level-settings Specification

## Purpose
The level's own settings the game keeps in its header, chosen in the level editor: how much
time Mario has, which music plays and how dark the background is.

## Requirements

### Requirement: Start time
The level panel SHALL show the level's start time and let the user change it, from 100 to
900 in steps of 100 (the game keeps the hundreds). After a save the game SHALL begin the level with that time.
Undo, redo, restore, level files and project files SHALL keep it.

#### Scenario: Shorter time
- **WHEN** level 01's time is set to 200, the copy is saved and level 01 is entered
- **THEN** the timer starts at 200

### Requirement: Music
The level panel SHALL let the user choose the level's music among the musics the game uses
for its levels, each named by a level that plays it. After a save the game SHALL play the
chosen music in that level, also after the star and after losing a life. Undo, redo, restore
and files SHALL keep it.

#### Scenario: Another music
- **WHEN** level 01 is given the music of a castle level and the copy is saved
- **THEN** level 01 plays the castle music

### Requirement: Listening in the editor
The level panel SHALL have a button to listen to the chosen music in the editor, played by the
game's own sound code, until it is pressed again, another level is shown or the copy is
saved; and a listening volume (an editor setting kept in `editor.cfg`, not the game's).

#### Scenario: Listen in the editor
- **WHEN** the listen button is pressed with the castle music chosen
- **THEN** the editor plays the castle music until the button is pressed again

#### Scenario: Quieter
- **WHEN** the listening volume is lowered and the editor is opened again
- **THEN** the music plays quieter and the game's own volume is as it was

### Requirement: Background shades
The level panel SHALL let the user choose how the level's background is shaded among the
game's own choices (normal and the two darker ones its levels use), drawn that way in the
editor at once. After a save the game SHALL show the level with those shades. Undo, redo,
restore and files SHALL keep it.

#### Scenario: Darker background
- **WHEN** level 01 gets the darkest shades and the copy is saved
- **THEN** the editor and the game show level 01 darker
