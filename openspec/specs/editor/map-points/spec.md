# editor/map-points Specification

## Purpose
How many points the world map and the game's zones can have, how the editor shows that
count, and moving their points by dragging, as in new zones.

## Requirements

### Requirement: More points on the world map
The Points tab SHALL let the user add new points to the world map up to a limit well above
the game's 6 free numbers: at least 40 new points. Each new point SHALL open a level, have
paths, a door and its own cleared state, as the game's points do.

#### Scenario: Add a seventh point
- **WHEN** the world map already has 39 new points and the user adds another
- **THEN** the point is added and works like the others

#### Scenario: No more points
- **WHEN** the world map has as many points as it can have and the user chooses New point
- **THEN** no point is added and the status bar says how many the world map can have

### Requirement: New points in the game's zones
The Points tab SHALL let the user add new points to the game's zones (Tree, Pumpkin, Macro,
Space, Mario, Turtle and the others with points) until each has as many points in all as a new
zone can have (at least 40), each opening a level, with paths, exits and its own cleared state,
as the zone's own points. Points numbered beyond the game's own SHALL NOT be offered as where a
door, an exit or a secret exit leads.

#### Scenario: A new point in the Tree Zone
- **WHEN** the Tree Zone is shown and the user chooses New point and clicks on the picture
- **THEN** a new point is there, opening no level, with no paths yet, and it is part of the Tree Zone

### Requirement: Paths shut until a level is cleared
The Points tab SHALL let the user shut each path of a point added beyond the game's numbers
(on the world map, a game zone or a new zone) until the level it opens is cleared, as the
game shuts its own zone paths. The choice SHALL be undoable and kept in the copy and in
project and zone files. A path drawn from a point that stands on the road of a way of the
point it reaches SHALL replace that way, so Mario stops at the new point.

#### Scenario: A shut path
- **WHEN** world point $80 opens a level and its path down is shut, and Mario stands on $80
- **THEN** pressing down does nothing until he clears $80's level, and then he walks down

#### Scenario: A point put on a path
- **WHEN** a path runs from $7B down to $82, a point $80 is put on it and a path is drawn from $80 up to $7B
- **THEN** $7B's way down leads to $80

### Requirement: Changed game levels and maps are marked
The editor SHALL mark as "mod" each game level and each game map (the world map and the
game's zones) whose saved copy differs from the original: a level's blocks, objects, start or
secret exit target; a map's picture, its own points (paths, places, levels) or points added
to it. Changes not yet saved SHALL NOT mark them.

#### Scenario: A saved block
- **WHEN** a block of level 3 is changed and the copy is saved
- **THEN** level 3's button and title show "mod" and the other untouched levels do not

### Requirement: Points used and points available
The Points tab SHALL show, for the map shown (the world map, a game zone or a new zone), how
many points it has and how many it can have, as "points used / maximum", and SHALL update it
as points are added and deleted.

#### Scenario: Counter on a new zone
- **WHEN** a new zone made from the Tree Zone with 10 points is shown
- **THEN** the panel shows "Points: 10 / <its maximum>"

#### Scenario: Counter follows changes
- **WHEN** a point is added to the world map
- **THEN** the world map's count goes up by one at once

### Requirement: Moving points on every map
On the world map and on the game's zones the Points tab SHALL move a point by dragging it,
as on new zones: Mario stands at the new place when he is on that point, and the paths to and
from it are redrawn straight to the new place. The map's picture SHALL NOT change (the point's
drawn mark stays where it was painted). Moving SHALL be undoable and undone in one step per
drag.

#### Scenario: Drag a world map point
- **WHEN** world map point $07 is dragged 40 pixels to the right and the copy is saved
- **THEN** in the game Mario walks to the new place on his ways to and from $07 and stands there

#### Scenario: Undo a drag
- **WHEN** the user presses Undo right after dragging a point of the Macro Zone
- **THEN** the point and its paths are back as they were
