# game/cheats Specification

## Purpose
A debug and cheats mode in the game port, for trying levels and maps made in the editor
quickly: turning it on, and the cheats and debug tools it offers in the pause menu.

## Requirements

### Requirement: Turning the cheats mode on at the start screen
The start screen where the cartridge is chosen SHALL have a "Debug / cheats" check box,
off until the person turns it on. The game started from that screen SHALL have the cheats
mode on only when the box is checked. The choice SHALL be remembered for the next start.

#### Scenario: Check the box
- **WHEN** the person checks "Debug / cheats" and plays the offered cartridge
- **THEN** the pause menu has the "Debug / cheats" entry

#### Scenario: Remembered
- **WHEN** the box was checked and the game is closed and opened again
- **THEN** the box is still checked

### Requirement: Cheats on when played from the editor
The game started from the editor's Play SHALL have the cheats mode on, whatever the start
screen's box says, and SHALL leave that box's remembered choice as it was.

#### Scenario: Play a level from the editor
- **WHEN** the person presses Play (F5) in the editor
- **THEN** the game's pause menu has the "Debug / cheats" entry

### Requirement: The cheats entry in the pause menu
With the cheats mode on, the pause menu SHALL have a "Debug / cheats" entry that opens a
list of the cheats; with it off the entry SHALL NOT be there. Cheats that only make sense in
a level SHALL be shown but not usable elsewhere.

#### Scenario: Cheats off
- **WHEN** the game was started from the start screen with the box unchecked
- **THEN** the pause menu has no "Debug / cheats" entry

#### Scenario: On the map
- **WHEN** the pause menu is opened on the world map and "Debug / cheats" is chosen
- **THEN** the list shows every cheat, and "Finish the level", "Secret exit" and "Summon an item" cannot be chosen

### Requirement: Invincible
The "Invincible" cheat SHALL be turned on and off from the list. While it is on, enemies,
projectiles and hazards that hurt Mario SHALL NOT make him lose his power-up or a life;
falling off the level SHALL still lose a life.

#### Scenario: Touch an enemy
- **WHEN** Invincible is on and small Mario walks into a Goomba
- **THEN** Mario stays as he was and the level goes on

#### Scenario: Turned off
- **WHEN** Invincible is turned off and small Mario walks into a Goomba
- **THEN** he loses a life, as in the game

### Requirement: Finish the level
In a level, the "Finish the level" cheat SHALL end the level at once as reaching its goal
with the bell rung does: the bonus game follows, then the map with the level cleared.

#### Scenario: Finish
- **WHEN** "Finish the level" is chosen in level 1
- **THEN** the bonus game begins, and afterwards Mario is on the map with level 1 cleared

### Requirement: Take the secret exit
In a level that has a secret exit, the "Secret exit" cheat SHALL end the level at once as
reaching its secret exit does: Mario goes to the map point the secret exit leads to (the
game's own, or the one set in the editor). In a level without one it SHALL be shown but not
usable.

#### Scenario: Secret exit
- **WHEN** "Secret exit" is chosen in a level with a secret exit
- **THEN** Mario is on the map at the point the secret exit leads to, not the one the goal leads to

#### Scenario: No secret exit
- **WHEN** the cheats list is opened in a level without a secret exit
- **THEN** "Secret exit" cannot be chosen and the list says the level has none

### Requirement: Summon an item
In a level, the "Summon an item" cheat SHALL let the person choose among the star, the
mushroom, the fire flower, the carrot and the heart, shown with the game's own pictures, and
SHALL put the chosen item in the level just in front of Mario, where he can take it. Taking
it SHALL do what it does in the game (the heart gives a life).

#### Scenario: Summon a fire flower
- **WHEN** big Mario summons the fire flower and walks into it
- **THEN** he becomes fire Mario

#### Scenario: Summon a heart
- **WHEN** Mario summons the heart and takes it
- **THEN** he has one life more

### Requirement: Freeze the timer
The "Freeze the timer" cheat SHALL be turned on and off from the list. While it is on, the
level's timer SHALL NOT go down.

#### Scenario: Frozen
- **WHEN** Freeze the timer is on and the level is played for a minute
- **THEN** the timer shows the same number as when the cheat was turned on

### Requirement: Lives, coins and golden coins
The list SHALL have "Lives to the maximum" (99 lives), "Coins to the maximum" (999 coins)
and "Six golden coins" (the six golden coins, as if every zone's boss were beaten). Each
SHALL take effect at once and stay as in the game from then on.

#### Scenario: Golden coins
- **WHEN** "Six golden coins" is chosen and Mario walks to the door of Wario's castle and presses A
- **THEN** the door opens as when the six coins were won

#### Scenario: Lives
- **WHEN** "Lives to the maximum" is chosen
- **THEN** the lives shown are 99

### Requirement: Info panel
The "Info panel" cheat SHALL be turned on and off from the list. While it is on, the game
SHALL draw over the picture the level number, Mario's position in the level, the map point
and the game's mode, kept up to date every frame.

#### Scenario: In a level
- **WHEN** the Info panel is on and Mario walks right
- **THEN** the position shown grows as he moves

### Requirement: Cheats leave the cartridge alone
Cheats SHALL change only the game being played; the cartridge file, the editor's copies and
the editor's files SHALL NOT be changed by them. The battery save SHALL keep what the game
itself saves (lives, coins and golden coins given by a cheat are saved as the game saves
them).

#### Scenario: Copy unchanged
- **WHEN** Invincible and Freeze the timer were used and the game is closed
- **THEN** the cartridge copy that was played is byte for byte as before

### Requirement: Go to a level
The cheats list SHALL have "Go to level", which lets the person choose any level the copy
has (the game's 32 and the editor's new ones, by number and name where it has one) and SHALL
start that level from the map as entering it from its own point does. It SHALL be usable on
the map and in a level (leaving the level being played without losing a life).

#### Scenario: From the map
- **WHEN** "Go to level" is opened on the world map and level 12 is chosen
- **THEN** level 12 begins, and finishing it brings Mario back to the map

#### Scenario: A new level
- **WHEN** a new level of the editor is chosen
- **THEN** that new level is played, not its base level

### Requirement: Open the whole map
The cheats list SHALL have "Open the whole map", which marks every level of the copy cleared
as the game marks them, so every path and point of the maps can be used. It SHALL take
effect at once and be saved as the game saves cleared levels. Like the six golden coins, it
marks the bosses beaten too.

#### Scenario: Open map
- **WHEN** "Open the whole map" is chosen on the world map
- **THEN** Mario can walk to every point of the world map and of the zones

### Requirement: Active objects view
The cheats list SHALL have "Objects view", turned on and off. While on, the game SHALL draw
over each active object (enemy, item, platform...) its type number in hexadecimal, kept up
to date every frame, and the info panel SHALL also show how many of the game's active object
places are in use.

#### Scenario: Enemies labelled
- **WHEN** Objects view is on and a Goomba walks on the screen
- **THEN** its type number is drawn over it and moves with it

### Requirement: Restart level
In a level, the game's menu SHALL offer Restart level (just above Back to map when that is offered) when the cheats mode is on or the game is a test game started from the editor (Play or Play from here). After its confirmation it SHALL leave the level as the game's pause menu does (no life lost) and enter the same level again: in a test game at the place it was played from (the Play from here place while it is the level first entered), otherwise at the level's own start.

#### Scenario: Play from here
- **WHEN** level 03 is played from here at x 1280, Mario walks right and the user chooses Restart level
- **THEN** level 03 begins again with Mario at x 1280 and the lives unchanged

#### Scenario: A new level
- **WHEN** new level 32 is played from the editor and the user chooses Restart level
- **THEN** new level 32 begins again at its start

#### Scenario: A plain game
- **WHEN** the cheats mode is off and the game was not started from the editor
- **THEN** the menu has no Restart level

### Requirement: Mario's block in the info panel
The info panel SHALL show, besides the level, position, map point and mode, the editor's column and row of the block Mario stands on (as the level editor and Play here count them).

#### Scenario: Standing on a block
- **WHEN** the info panel is on and Mario stands on the block at column 14, row 30
- **THEN** the panel shows EDITOR 14,30

### Requirement: No Back to map in a level tried from the editor
In a test game started on a level from the editor (Play or Play from here), the game's menu SHALL not offer Back to map; the other ways of leaving (Restart level, finishing, Go to level, File select, Quit) stay.

#### Scenario: Play from here
- **WHEN** level 03 is played from here and the player opens the menu in the level
- **THEN** Restart level is there and Back to map is not

#### Scenario: Playing the copy's map
- **WHEN** the zone editor's Play starts the game on the world map and the player enters a level
- **THEN** the menu has Back to map as usual

### Requirement: No File select in a test game
In any test game started from the editor (a level's Play or Play from here, or the zone editor's Play on the copy's map), the game's menu SHALL not offer File select, in a level or on a map.

#### Scenario: The zone editor's Play
- **WHEN** the zone editor's Play starts the game and the player opens the menu on the world map
- **THEN** File select is not there
