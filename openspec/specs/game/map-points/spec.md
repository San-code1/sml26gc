# game/map-points Specification

## Purpose
Points added beyond the numbers the game leaves free (on the world map, the game's zones and
new zones), played by the game's own map code in this port with their own progress.

## Requirements

### Requirement: Playing added points
The game SHALL let Mario walk to, stand on and enter the level of every point the editor
added (world map, game zones, new zones), and SHALL open their paths when their level is
cleared, as with the game's own points. The game's own points SHALL work as before.

#### Scenario: Walk to an added point
- **WHEN** the world map has a 10th new point joined by a path to point $07, and Mario walks there and presses A
- **THEN** he stands on it and its level begins

#### Scenario: Clear an added point
- **WHEN** the level of an added point in the Pumpkin Zone is cleared
- **THEN** that point shows cleared, its paths open and no other point changes

### Requirement: Progress of added points
The cleared state of added points SHALL be saved and resumed with the game file, as the
game's own points are; a copy without added points SHALL save exactly as before.

#### Scenario: Resume
- **WHEN** an added point was cleared, the game was saved and later resumed
- **THEN** the point is still cleared and its paths open
