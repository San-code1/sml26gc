# editor/play-from-here Specification

## Purpose
Starting the game in the shown level at a chosen place, to try a part of a level without
walking to it, and without changing where the level really begins.

## Requirements

### Requirement: Play from here
The level editor SHALL have "Play from here" (Shift+F5, and a button in the level panel):
it shows Mario's outline under the mouse, and a click on the level saves the copy as Play
does and starts the game in the shown level with Mario standing on the clicked block, with
the cheats mode on (Esc, a right click or the button again cancels). A place where Mario
cannot stand (inside solid blocks, outside the level) SHALL be refused with a message in the
status bar.

#### Scenario: Nothing painted
- **WHEN** Play from here is clicked on a level in Blocks mode with the Pencil chosen
- **THEN** the level is not changed by that click

#### Scenario: Try the end of a level
- **WHEN** Shift+F5 is pressed and level 03 is clicked near its finish
- **THEN** the game starts level 03 with Mario there, and the level's start in the editor is where it was

### Requirement: The level's own start is kept
Play from here SHALL NOT change the level's saved start; entering the level any other way, or
again after losing a life, SHALL use its own start.

#### Scenario: Losing a life
- **WHEN** Mario loses a life in a level started with Play from here
- **THEN** he begins again at the level's own start (or its checkpoint), as the game does

#### Scenario: A new level
- **WHEN** Play from here is used in a new level of the editor
- **THEN** that new level is played with Mario at the chosen place
