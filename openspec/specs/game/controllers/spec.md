# game/controllers Specification

## Purpose
Playing the game with a controller (gamepad) as well as the keyboard: the bindings of each,
setting them, and plugging and unplugging the controller.

## Requirements

### Requirement: Keyboard and controller together
The keyboard SHALL always play, with or without a controller. A connected controller SHALL
play too, at the same time, each with its own bindings. Unplugging the controller SHALL leave
the game playing from the keyboard without stopping it, and a controller plugged in SHALL play
at once.

#### Scenario: Both
- **WHEN** a controller is connected and a level is played
- **THEN** Mario moves with the keyboard's arrows and with the controller's D-pad

#### Scenario: Unplugged
- **WHEN** the controller is unplugged in a level
- **THEN** the game goes on and the keyboard moves Mario; plugging it back makes it play again

### Requirement: Choosing which bindings to set
The pause menu's Controls SHALL begin with an Input row, Keyboard or Controller (Left/Right or
OK change it), kept in `settings.cfg`, Keyboard by default. It SHALL only choose which
bindings the screen lists, sets and restores (Restore puts back that input's defaults only); it
SHALL NOT change what plays.

#### Scenario: The controller's list
- **WHEN** Input is changed to Controller in Controls
- **THEN** the list shows the controller's button for each binding, and the keyboard still plays

### Requirement: Binding controller buttons
With Controller chosen in Controls, every binding (Up, Down, Left, Right, A, B, Start, Select,
save state, load state, rewind, fast forward, menu) SHALL be set by choosing it and pressing a
controller button, or a direction of the left stick; a button in use by another binding SHALL
swap with it, so no binding is lost. The keyboard's Esc SHALL cancel the wait.

#### Scenario: Another A button
- **WHEN** A is chosen in Controls with Controller chosen and the controller's Y button is pressed
- **THEN** Y is the A button from then on, and the binding that had Y takes A's old button

### Requirement: The menu binding
Opening the pause menu SHALL be a binding of its own, for the keyboard (Esc by default) and the
controller (Select, called Back, View or Share on some controllers, by default), set in
Controls like the others. It SHALL open the menu anywhere and go back in it; in a level the
Start binding SHALL also open it. A key or button may not open the menu and do another
binding's job (choosing one in use swaps them, as for every binding).

#### Scenario: Another menu key
- **WHEN** the keyboard's menu binding is set to M
- **THEN** M opens the pause menu on the map and in a level, and Esc no longer does

#### Scenario: Menu from the controller
- **WHEN** the controller's Select is pressed on the world map with the defaults
- **THEN** the pause menu opens

### Requirement: Controller defaults
The controller's default bindings SHALL follow the buttons' places on a Game Boy: the D-pad
for the directions, the right face button for A and the bottom one for B, Start for Start,
the left face button for Select, Select (Back, View, Share) for the menu, the left and right
shoulder buttons for save state and load state, the left trigger for rewind and the right
trigger for fast forward. The left stick SHALL also move like the D-pad unless one of its
directions is bound to something else.

#### Scenario: Defaults
- **WHEN** an Xbox controller is used with the defaults
- **THEN** B (right) jumps as the Game Boy's A, A (bottom) is the Game Boy's B, X is Select, View opens the menu, and the left stick walks

### Requirement: The pause menu with a controller
A connected controller SHALL move through the pause menu as the keyboard does (D-pad or left
stick to move and change values, the bound A to choose, the bound B and the menu binding to go
back); the controller's guide button, when it has one, SHALL open the menu anywhere.

#### Scenario: Volume from the controller
- **WHEN** Start is pressed on the controller in a level, then Sound is chosen and Right pressed
- **THEN** the volume goes up, as with the keyboard

### Requirement: A key to restart the level
The game SHALL have a bindable Restart level key (F8 and the controller's north face button by default), shown in Controls; in a level, with the cheats mode on or in a test game, it SHALL restart the level at once as the menu's Restart level does, without asking.

#### Scenario: F8 in a test game
- **WHEN** a test game from Play from here is in its level and the player presses F8
- **THEN** the level begins again at the chosen place, with no life lost

#### Scenario: A plain game
- **WHEN** the cheats mode is off in a game not started from the editor and the player presses F8
- **THEN** nothing happens
