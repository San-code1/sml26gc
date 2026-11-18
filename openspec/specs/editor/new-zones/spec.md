# editor/new-zones Specification

## Purpose
The world map editor makes new zones from the game's zones and edits them like the game's:
their picture, points, paths, the level each point opens, their exits and the door into them.

## Requirements

### Requirement: Making a new zone
The world map editor SHALL make a new zone from the zone shown (one of the game's or a new
one): a copy of its picture, points, paths, levels and exits as edited, with the same base,
and SHALL show it. It SHALL refuse when there are as many new zones as there can be.

#### Scenario: New zone from the Tree Zone
- **WHEN** the Tree Zone is shown and the user makes a new zone
- **THEN** "New zone 1 (Tree Zone)" is shown, looking and playing as the Tree Zone does now, and it is in the map list after the game's zones

### Requirement: New zones in the map list
New zones SHALL be shown, painted and edited like the game's zones: PgUp and PgDn and the map
arrows SHALL go through them, and their names SHALL say their base zone.

#### Scenario: Stepping through maps
- **WHEN** there are two new zones and the user presses PgDn on the Bonus game map
- **THEN** new zone 1 is shown, then new zone 2, then the world map

### Requirement: Points of a new zone
In a new zone the Points tab SHALL add a point where the user clicks, move a point by dragging
it, and delete the chosen point with its paths, up to at least 40 points per new zone (its
base zone's numbers plus numbers the game does not use); each point opens a level, has paths
and can be an exit, as on the game's zones.

#### Scenario: Add a point
- **WHEN** a new zone made from the Tree Zone has its 9 points and the user chooses New point and clicks on the picture
- **THEN** a tenth point is there, opening no level, with no paths yet

#### Scenario: More points than before
- **WHEN** a new zone made from the Tree Zone already has 20 points and the user adds another
- **THEN** the point is added and works like the others

#### Scenario: No more points
- **WHEN** a new zone already has as many points as it can have and the user chooses New point
- **THEN** no point is added and the status bar says at once how many a zone can have

### Requirement: Naming a new zone
The world panel SHALL show a name field on a new zone; the name (up to 24 printable ASCII
characters) SHALL be shown in the map list with the base zone's name, saved in the edited copy
and kept in zone and project files. A zone with no name SHALL be shown as "New zone N".

#### Scenario: Name a zone
- **WHEN** the user clicks the name field of new zone 1, types "Forest" and presses Enter
- **THEN** the map list shows "Forest (Tree Zone)" and, after saving and opening again, still does

### Requirement: Restoring what is being edited
The Restore button (formerly Original) SHALL put back, undoably, only the level or map shown
as the game has it: a level's blocks and objects, a map's picture and points. A new level or
new zone SHALL be restored as its base is in the game, keeping its name (and a zone its door).

#### Scenario: Restore the world map
- **WHEN** the world map has new points and a door into a new zone and the user chooses Restore
- **THEN** the new points and that door are gone and the world map is as in the game

#### Scenario: Restore a zone only
- **WHEN** the world map and the Tree Zone have both been changed, the Tree Zone is shown and the user chooses Restore
- **THEN** the Tree Zone's picture and points are as in the game and the world map keeps its changes

#### Scenario: Restore a new zone
- **WHEN** a new zone made from the Pumpkin Zone, with points added and a door on $07, is shown and the user chooses Restore
- **THEN** its picture and points are the Pumpkin Zone's in the game, and its door is still $07

### Requirement: Choosing where a door or exit leads on the maps
Where a door or an exit leads SHALL be chosen by looking, not by stepping through a list:
Choose on the map shows the maps it can lead to as tabs over the map (the world map for
exits, the zones a door enters, the new zones), and a click picks a world map point, or the
point of a zone Mario enters on, which becomes that zone's entry point (for every door and
exit into it, game zones included). Esc or right click cancels; a door can be set to none.

#### Scenario: Exit to a world point
- **WHEN** a Tree Zone exit point is chosen, the user chooses Choose on the map and clicks world point $07
- **THEN** the exit leads to $07 and the Tree Zone is shown again

#### Scenario: Door into a new zone by its tab
- **WHEN** world point $6D is chosen, the user chooses Choose on the map, picks the tab of new zone 1 and clicks its point $24
- **THEN** $6D's door enters new zone 1, and $24 is its entry point

#### Scenario: Entry of a game zone
- **WHEN** the user chooses the Tree Zone's point $21 as where a door leads and plays it
- **THEN** every door into the Tree Zone enters it on $21, Mario walking onto it

### Requirement: Door into a new zone
In the Points tab the door of any world map point SHALL be able to lead into a new zone, and a
zone's exit SHALL be able to lead into a new zone, as into the game's zones.

#### Scenario: Door to a new zone
- **WHEN** world point $05 is chosen and the user steps its door to "New zone 1 (Tree Zone)"
- **THEN** point $05 enters new zone 1 when played in the port

### Requirement: Deleting a new zone
The editor SHALL delete the new zone shown; doors and exits that led into it SHALL lead
nowhere, the new zones after it SHALL move down a number, and the game's zones SHALL NOT be
deleted.

#### Scenario: Delete
- **WHEN** new zone 1 is shown and deleted while point $05 led into it
- **THEN** new zone 2 becomes new zone 1 and point $05 has no door

### Requirement: Saving new zones
New zones SHALL be saved in the edited copy with every other change and read back when it is
opened again; a save SHALL say when they do not fit.

#### Scenario: Save and open again
- **WHEN** a project with two new zones is saved and the editor is opened again
- **THEN** both new zones are there as they were saved

### Requirement: Undo in new zones
Painting a new zone and changing its points, paths, levels, exits and doors SHALL be undoable
like the same changes on the game's zones.

#### Scenario: Undo a point
- **WHEN** a point was added to a new zone and the user presses Undo in the Points tab
- **THEN** the point is gone

### Requirement: Blank new zone
The world map editor SHALL make a blank new zone, like a blank level: the user chooses the
game zone whose tiles and numbers it uses (one a door enters), and the new zone starts with an
empty picture and only its entry point (where the base zone's is), with no paths, levels or
exits. Making a new zone as a copy of the zone shown SHALL stay.

#### Scenario: A blank zone over the Pumpkin Zone's tiles
- **WHEN** New blank zone is chosen and the Pumpkin Zone picked
- **THEN** a new zone appears with an empty picture drawn with the Pumpkin Zone's tiles and a single point, its entry

### Requirement: Mario's start copied with a zone
Making a new zone as a copy of the zone that holds Mario's start SHALL ask whether the start
moves to the copy or stays.

#### Scenario: Copying the start's zone
- **WHEN** Mario's start is on the Tree Zone and New zone is made from it
- **THEN** the editor asks whether the start goes to the new zone or stays on the Tree Zone
