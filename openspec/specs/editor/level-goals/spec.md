# editor/level-goals Specification

## Purpose
Where a level ends and the other ways out of it (a secret exit, the bonus bell): shown and
placed in the level editor and played by the game's own code.

## Requirements

### Requirement: Showing and placing the finish
The level editor SHALL show where the shown level ends (its goal block) and SHALL let the user
place it with "Place finish" the same way as the start: a click puts it there, a drag moves
it, undo and redo work and the level is marked changed; the cell it leaves gets back the
block it had. Boss levels SHALL say they have no goal. After a save, touching the placed
finish SHALL clear the level as the game's own goal does. The gate is only a picture.

#### Scenario: The cell left behind
- **WHEN** the game's goal of level 00 is moved
- **THEN** its old cell gets the block to its left, and a later move puts back what that cell had

#### Scenario: Move the finish
- **WHEN** level 01's finish is placed half way through the level and the copy is saved
- **THEN** in the game, touching the finish clears level 01 and Mario is back on the map with its paths open

#### Scenario: A boss level
- **WHEN** a level that ends with a boss is shown
- **THEN** no finish is shown and the panel says the level ends with its boss

### Requirement: Secret exit
Where the game's own mechanism allows it, the level editor SHALL let the user give a level a
secret exit with "Place secret exit", shown and placed like the finish. Levels that cannot
have one SHALL say why instead of offering the action. Taking it in the game SHALL work as the
game's secret exits do: the level is not marked cleared, the secret exit's way is opened and
Mario is taken where the secret exit leads.

#### Scenario: A secret exit in an edited level
- **WHEN** a level that can have a secret exit is given one, the copy is saved and Mario reaches it
- **THEN** Mario is taken where the secret exit leads and the level's own point is not marked cleared

### Requirement: Where a secret exit leads
The level panel SHALL let the user choose where a level's secret exit leads, on the maps, as
doors and exits are chosen: a world map point, a point of a game zone or a point of a new
zone (which becomes that zone's entry), in any zone. Until the user chooses, a game level's
secret exit SHALL lead where the game's own does. The choice SHALL be undoable and kept in
level and project files.

#### Scenario: Secret exit into a new zone
- **WHEN** a level's secret exit is set to lead into new zone "Sky Zone" on its point $4C, the copy is saved and Mario takes it
- **THEN** Mario is on "Sky Zone"'s map, on point $4C

#### Scenario: Secret exit to the world map
- **WHEN** a level's secret exit is set to lead to world map point $07 and Mario takes it
- **THEN** Mario is on the world map, on point $07

#### Scenario: The game's own secret exit
- **WHEN** a game level with a secret exit is shown and its secret exit was never changed
- **THEN** the panel shows where the game's secret exit leads, and in the game it works as before

### Requirement: Secret exits into a game zone
A secret exit into a game zone SHALL put Mario on the chosen point and leave him there (he
SHALL NOT walk on as after the zone's own levels). A world point drawn on a zone's map SHALL
be offered there too. Points that open a boss level SHALL NOT be offered as targets.

#### Scenario: Into the Macro Zone
- **WHEN** a level's secret exit is set to lead to the Macro Zone's point $34 and Mario takes it
- **THEN** the Macro Zone is shown with Mario on $34, and he stays there

#### Scenario: No boss point
- **WHEN** the user chooses where a secret exit leads and points at a zone's boss point
- **THEN** the point cannot be chosen

### Requirement: Bonus exit
A placed finish SHALL keep the bonus way out the game's goals have: the level's bell (the
object above the goal that leads to the bonus game) SHALL move with the finish, keeping its
place relative to it, so that ringing it plays the bonus game and then goes on as after the
game's own goals. A level without a bell keeps none.

#### Scenario: Ring the bell of a moved finish
- **WHEN** level 01's finish was moved, the copy is saved and Mario rings the bell that moved with it
- **THEN** the bonus game is played and then Mario is back on the map with level 01 cleared

### Requirement: Placing checkpoints
The level editor SHALL show a level's midway bells (checkpoints), each with the place Mario
starts again after ringing it, and SHALL let the user add one with "Place checkpoint" (K,
and a button beside "Place finish"): a click puts a bell at that cell and its restart place
just left of it on the ground below. Dragging a bell moves it with its restart place, deleting
it removes the checkpoint; undo and redo work and the level is marked changed.

#### Scenario: Move a checkpoint
- **WHEN** level 01's checkpoint is dragged further on, the copy is saved, Mario rings it and loses a life
- **THEN** he starts again next to the moved bell

### Requirement: Several checkpoints per level
A level SHALL have up to 4 checkpoints. "Place checkpoint" in a level that has 4 SHALL say so
and add nothing. Level 31 SHALL say it cannot have checkpoints (the game has no restart
place for it) instead of offering the action.

#### Scenario: Two checkpoints
- **WHEN** level 01 gets a second checkpoint further on
- **THEN** both bells and both restart places are shown and the level has two checkpoints

#### Scenario: The limit
- **WHEN** a level with 4 checkpoints gets "Place checkpoint"
- **THEN** nothing is added and the status bar says the level has the most checkpoints it can have

### Requirement: Playing the checkpoints
After a save, losing a life after ringing bells SHALL start Mario again at the restart place
of the last bell he rang, in the game's levels and in new levels. Mario SHALL keep that
checkpoint until he rings another one or the level is finished, as the game keeps its own.
A level with one checkpoint SHALL play exactly as the game's own checkpoints do.

#### Scenario: The last bell rung
- **WHEN** Mario rings level 01's first and then its second bell and loses a life
- **THEN** he starts again next to the second bell

#### Scenario: Add a checkpoint to a new level
- **WHEN** a new level is given a checkpoint, the copy is saved, Mario rings it and loses a life
- **THEN** he starts again next to it, and the base level's checkpoints are unchanged

### Requirement: The checkpoint in files and restore
The checkpoints (their bells and restart places) SHALL be kept in level files and project
files and come back with them; restore SHALL put back the game's own checkpoint (for a new
level, its base level's).

#### Scenario: Export and import
- **WHEN** a level whose checkpoint was moved is exported and imported as a new level
- **THEN** the new level's checkpoints are at the same places
