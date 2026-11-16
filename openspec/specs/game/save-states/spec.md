# game/save-states Specification

## Purpose
Saving and loading the running game at once, and rewinding it, so a part of a level can be
tried again without replaying what comes before it. Only in the cheats mode.

## Requirements

### Requirement: Save and load the game at once
With the cheats mode on, the save state key (F6 unless changed) SHALL save the whole running
game (the console as it is at that moment) and the load state key (F7 unless changed) SHALL
bring it back exactly, wherever the game is (a level, the map). With the cheats mode off those
keys SHALL do nothing. A short note on the screen SHALL say "Saved" or "Loaded", or that
nothing was saved yet.

#### Scenario: Load after losing a life
- **WHEN** F6 is pressed in level 1, then Mario loses a life and F7 is pressed
- **THEN** the game is back in level 1 where F6 was pressed, with the lives, coins, power-up and time it had then

#### Scenario: Nothing saved
- **WHEN** F7 is pressed before F6 was ever pressed
- **THEN** the game goes on unchanged and the note says nothing was saved

#### Scenario: Cheats mode off
- **WHEN** the game was started without the cheats mode and F6 then F7 are pressed
- **THEN** nothing happens

### Requirement: A saved game stays in memory
A saved game SHALL last until the game is closed or a new one is saved over it; it SHALL NOT
be written to any file, and the battery save SHALL keep working as before.

#### Scenario: Closed
- **WHEN** a game was saved with the save state key and the game is closed and opened again
- **THEN** the load state key says nothing was saved, and the battery save has the progress the game saved itself

### Requirement: Keys for save states and rewind
The save state, load state and rewind keys SHALL be set in the pause menu's Controls with the
Game Boy buttons, kept in `settings.cfg`, and never share a key with another binding (choosing
a key in use swaps the two, as for the buttons).

#### Scenario: Another save key
- **WHEN** the save state key is set to F9 in Controls
- **THEN** F9 saves the game and F6 does nothing more

### Requirement: The keys shown when a level begins
The tag shown for a few seconds at start-up and when a level begins SHALL say which key opens
the menu (Start in a level, Esc elsewhere) and, in the cheats mode, the save state, load state
and rewind keys as they are set; it SHALL NOT repeat a second way of opening the menu.

#### Scenario: Cheats mode on
- **WHEN** a level begins with the cheats mode on and the default keys
- **THEN** the tag reads "ENTER: MENU", "F6: SAVE", "F7: LOAD", "R: REWIND"

### Requirement: Rewind
With the cheats mode on, holding the rewind key (R unless changed) SHALL play the game backwards, at the game's speed, up to
the last 10 seconds of play; letting go SHALL go on playing from there. Time spent in the
pause menu SHALL NOT count. Rewinding SHALL stop at the oldest moment kept.

#### Scenario: Undo a fall
- **WHEN** Mario falls into a pit and R is held before the life is lost
- **THEN** the game goes back to before the fall and goes on from there when R is let go

#### Scenario: As far as it goes
- **WHEN** R is held for longer than the time kept
- **THEN** the game stays at the oldest moment kept until R is let go
