# editor/panel-labels Specification

## Purpose
How the editor's panels show names: the person's own levels and zones told apart from the
game's, and names too long for their place shown whole on demand.

## Requirements

### Requirement: The user's own levels and zones tagged
In the level picker's grid and in the zone editor's map list, every level made in the editor
(32 and up) and every new zone SHALL carry a tag that the game's own levels and zones do not
have, explained once next to the grid (for example "made in the editor"); the tooltip of such a
button SHALL say so too.

#### Scenario: A copied level
- **WHEN** a level is copied and the level grid is opened
- **THEN** the new level's button carries the tag and the game's 32 do not

#### Scenario: A new zone
- **WHEN** a blank zone is made and the map list is opened
- **THEN** its button carries the tag and the game's zones do not

### Requirement: Cut texts shown whole
A text the editor shortens to fit a button, a field or a row (shown with ".." or made too
small to read) SHALL show whole in a tooltip while the mouse is on it, in the level and zone
editors, the choosers and the catalogue.

#### Scenario: A long level name
- **WHEN** the mouse is on a level picker button whose name is cut
- **THEN** a tooltip shows the whole name

#### Scenario: A point's value
- **WHEN** the mouse is on the Points tab's Opens row whose level name is cut
- **THEN** a tooltip shows it whole
