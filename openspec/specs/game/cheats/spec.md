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
