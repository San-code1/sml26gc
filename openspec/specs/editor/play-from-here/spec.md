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
Play from here SHALL NOT change the level's saved start: entering the level any other way SHALL
use its own start. Losing a life in a game started with Play from here SHALL begin the level
again at the chosen place, every time (see Losing a life in a test game).

#### Scenario: Losing a life
- **WHEN** Mario loses a life in a level started with Play from here
- **THEN** he begins the level again at the place chosen with Play from here, and the level's start in the editor is where it was

#### Scenario: A new level
- **WHEN** Play from here is used in a new level of the editor
- **THEN** that new level is played with Mario at the chosen place

#### Scenario: Entering it another way
- **WHEN** in a game started with Play from here Mario goes back to the map with the menu and enters the level from its map point
- **THEN** he begins at the level's own start

### Requirement: A fresh test game
Play and Play from here SHALL start a test game of the project: no battery save read or
written, no new zone progress kept, and Mario starting with the lives (0-99) and coins (0-999)
set in the Level tab's Test game (99 and 999 at first, kept between starts of the editor), so
every Play is the same fresh game.

#### Scenario: Two plays
- **WHEN** a level is played from the editor, Mario loses lives and collects coins, and Play is pressed again
- **THEN** the new game starts again with the lives and coins set, and nothing of the first game kept

#### Scenario: Fewer lives to test
- **WHEN** the Test game is set to 5 lives and 120 coins and Play is pressed
- **THEN** the game starts with 5 lives and 120 coins

### Requirement: Losing a life in a test game
In a test game started from the level editor, losing a life with lives left SHALL enter the
tested level again (new levels included) instead of leaving Mario on the world map: with Play,
at the level's own start or its checkpoint; with Play from here, at the chosen place, each time.
Game over, finishing the level, its secret exit and the pause menu's Back to map and Go to level
SHALL behave as before.

#### Scenario: Try again
- **WHEN** level 03 is played from the editor with 5 lives and Mario falls into a pit
- **THEN** after the game's life-lost sequence level 03 begins again, with 4 lives

#### Scenario: Try again from the chosen place
- **WHEN** level 03 is played with Play from here near its finish and Mario loses a life twice
- **THEN** both times level 03 begins again with Mario at that place

#### Scenario: Last life
- **WHEN** Mario loses his last life in a test game
- **THEN** the game shows game over as it does today

#### Scenario: Level finished
- **WHEN** Mario reaches the finish of the tested level
- **THEN** the game goes back to the world map as it does today

#### Scenario: A new level
- **WHEN** new level 33 is played from the editor and Mario loses a life
- **THEN** level 33 begins again, not its base level

#### Scenario: Another level entered
- **WHEN** in a test game Mario goes back to the map with the menu, enters another level and loses a life there
- **THEN** that level begins again at its own start

### Requirement: A test game plays its level whatever the world map opens
A test game started on a level (Play or Play from here) SHALL enter that level, and enter it again after each lost life with lives left, whatever the project's world map points open: no point opening it, or no point opening any level. The project's map SHALL be unchanged by it.

#### Scenario: No point opens the level
- **WHEN** no world map point of the project opens level "test" (or the point where a new game starts opens no level) and the user plays "test" from the editor
- **THEN** the game starts in "test" instead of staying on the world map, and every lost life with lives left enters "test" again

#### Scenario: No point opens any level
- **WHEN** every world map point opens no level and the user plays level 05 from here
- **THEN** the game starts in level 05 at the chosen place, every lost life with lives left starts there again, and the map's points still open no level

### Requirement: The editor's game graphics whatever the project's start and map
The editor SHALL capture the world map's tiles and every level's graphics from a console where a new game begins on the world map's point $00 and that point opens a level, whatever start point and point levels the project has; the project's cartridge SHALL be unchanged by it.

#### Scenario: Mario starts on another map
- **WHEN** the project makes Mario start on another point (one that shows another map at boot) and the user opens the world map
- **THEN** the world map is drawn with its own tiles

#### Scenario: Point $00 opens no level
- **WHEN** the project's map point $00 opens no level and the user opens a level
- **THEN** the level's blocks and objects are drawn with the graphics the game loads for it

### Requirement: Choosing where to play shows what will not appear
When Play here is turned on in the level editor, in Blocks or Objects mode, the chosen block and object type SHALL be let go so nothing is painted or placed, and the band whose objects do not appear at the start SHALL follow the mouse, worked out from the start a click there would give, with its note; the level start's own band SHALL not be shown meanwhile.

#### Scenario: Play here while painting blocks
- **WHEN** block $12 is chosen in Blocks mode and the user presses Here
- **THEN** no block is chosen, and moving the mouse over the level moves the band of objects that will not appear with Mario's outline under the mouse

#### Scenario: Play here with objects
- **WHEN** an object type is chosen in Objects mode and the user presses Here
- **THEN** no type is chosen and the band follows the mouse instead of standing at the level's start

### Requirement: Mario under the mouse while choosing where to play
While Play here is on, the level editor SHALL draw Mario's own picture under the mouse where a click would start him, framed in yellow (the warning colour when he would stand inside blocks), instead of a bare box.

#### Scenario: Moving the mouse
- **WHEN** Play here is on and the mouse moves over the level
- **THEN** Mario's picture follows it, standing on the block under the mouse

### Requirement: Play here again
The level editor SHALL start the test game again at the last place Play here was used in the level shown with a shortcut (Ctrl+F5), without choosing it again; with no such place in that level it SHALL say so.

#### Scenario: Again
- **WHEN** the user played level 03 from here at a place, edited a block and presses Ctrl+F5
- **THEN** the copy is saved and the game starts at that same place

#### Scenario: Never played here
- **WHEN** Play here was never used in the level shown and the user presses Ctrl+F5
- **THEN** nothing starts and the status says to use Here first

### Requirement: Mario's power in a test game
The test game settings SHALL let the user choose how Mario starts: small, big, carrot or fire (kept in the editor's settings); the game SHALL give him that power each time it enters the tested level, the first entry, each one after a lost life and Restart level included.

#### Scenario: Fire Mario
- **WHEN** the test game's power is fire and the user plays level 03 from here
- **THEN** Mario starts as fire Mario, and again after losing a life

### Requirement: One test game at a time
While a test game started from the editor is still open, Play, Play here, Play here again and Play from a point SHALL start no other one, saying in the status to close it first; once it is closed they start a game again.

#### Scenario: Pressing Play twice
- **WHEN** the user presses Play and, with that game still open, presses Play here
- **THEN** no second game opens and the status says the game is already open

#### Scenario: After closing it
- **WHEN** the user closes the game window and presses Play
- **THEN** the game starts

### Requirement: The play buttons show an open game
While a test game started from the editor is open, the Play and Play here buttons of both editors and Play from a point SHALL be drawn faded, their tooltip saying the game is open; once it is closed they are as usual.

#### Scenario: Game open
- **WHEN** a test game is open and the user moves the mouse over Play
- **THEN** Play is faded and the tooltip says to close the game to play again
