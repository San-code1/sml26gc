# editor/level-settings Specification

## Purpose
The level's own settings, chosen in the level editor for the game's levels and new ones: how
much time Mario has, which music plays and how dark the background is (its header), and its
gravity, auto-scroll, camera margin and animated tiles (the game's per-level tables).

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

### Requirement: Gravity
The level's settings SHALL let the user choose its gravity, normal or space (the floating
jumps of the Space Zone). After a save the game SHALL play the level with that gravity.

#### Scenario: Space gravity in level 01
- **WHEN** level 01 gets space gravity, the copy is saved and level 01 is played
- **THEN** Mario jumps and falls as in the Space Zone

#### Scenario: Normal gravity in the Space Zone
- **WHEN** level 18 gets normal gravity and the copy is saved
- **THEN** Mario jumps in level 18 as in level 01

### Requirement: Auto-scroll
The level's settings SHALL let the user turn on auto-scroll, which the game has but none of its
levels uses: the screen moves right by itself and pushes Mario along, losing a life if he is
caught against a wall, and stops at the level's end. After a save the game SHALL play the
level that way.

#### Scenario: Finished while scrolling
- **WHEN** Mario reaches the finish of a level with auto-scroll
- **THEN** the level ends as usual and the map comes back

#### Scenario: Pushed along
- **WHEN** level 01 gets auto-scroll, the copy is saved and level 01 is played without touching the pad
- **THEN** the screen moves right by itself and Mario is pushed along by its left edge

### Requirement: Camera margin
The level's settings SHALL let the user choose the camera's vertical margin, normal or wide
(the camera moves up sooner, as in the game's levels that climb). After a save the
game SHALL follow Mario that way.

#### Scenario: Wide margin
- **WHEN** level 01 gets the wide margin and the copy is saved
- **THEN** the camera starts to move up when Mario is lower on the screen than before

### Requirement: Animated tiles
For a level whose tiles the game animates, the level's settings SHALL let the user keep the
animation or turn it off. Only the level's own animation SHALL be offered.

#### Scenario: Still tiles
- **WHEN** the animation of a level with animated tiles is turned off and the copy is saved
- **THEN** those tiles stay still in the game

### Requirement: Settings kept like the others
The new settings SHALL be undone, redone, restored, kept in level and project files, shown in
the level's "mod" mark when they differ from the game's, and played by new levels as by the
game's own.

#### Scenario: A new level with space gravity
- **WHEN** a new level based on level 01 gets space gravity and is saved and played
- **THEN** it plays with space gravity and level 01 keeps normal gravity
