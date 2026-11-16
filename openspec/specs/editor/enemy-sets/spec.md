# editor/enemy-sets Specification

## Purpose
A level's enemies chosen among the game's, from several levels, as long as they fit together
in the level's enemy sprite area, drawn by the game as in their own levels.

## Requirements

### Requirement: Enemies that fit together
For any level, new or one of the game's, the editor SHALL let the user choose its enemy set
(starting from the level's own enemies): enemies of the level's object catalogue whose sprite
tiles in the enemy area do not overlap, added from the object catalogue and removed in the
Level tab. An enemy that does not fit SHALL not be added, and the editor SHALL name the enemy
it clashes with.

#### Scenario: Two that fit
- **WHEN** level 01's frog is removed from its set and level 00's Koopa Troopa is added
- **THEN** the set has the porcupine, the bee and the Koopa Troopa, and the palette offers the Koopa Troopa

#### Scenario: Two that clash
- **WHEN** an enemy whose tiles overlap one already chosen is added
- **THEN** it is not added and the editor names the enemy it clashes with

### Requirement: Known before adding
The catalogue SHALL show, on each enemy of another level of the shown level's catalogue,
whether it fits the level's set (fits, no room, or still unknown while tiles are captured),
and, for the chosen one, which enemy it clashes with; Add SHALL be enabled only when it fits.

#### Scenario: Seen before pressing
- **WHEN** the catalogue is opened in a level whose frog fills the tiles the Koopa Troopa needs
- **THEN** the Koopa Troopa's cell says no room and its panel names the frog, with Add disabled

### Requirement: Enemies brought from other levels
The object palette SHALL mark the types of the level's set that come from another level, and
say which level when the mouse is over one. Remove beside the chosen object in the palette,
Remove from this level in the catalogue, a right click on it in the palette, or Remove in the
Level tab for any enemy of the set, SHALL take it out of the set and every object of its type
out of the level (they would look broken), in one undo step.

#### Scenario: Taking the Goblin out
- **WHEN** level 33's set has level 06's Goblin, placed twice, and Remove is pressed beside it in the palette (or Remove from this level in the catalogue)
- **THEN** the Goblin leaves the set and both are gone from the level, and Ctrl+Z brings them back

### Requirement: Played with the chosen enemies
After a save, the game SHALL play a level with a custom enemy set drawing each chosen enemy as
in its own level; undo, restore and level and project files SHALL keep the set, and a level
without one SHALL keep the game's.

#### Scenario: Drawn in the editor
- **WHEN** a level's set has level 06's Goblin
- **THEN** the editor draws it, in the level and in the palette, as in level 06

#### Scenario: Played
- **WHEN** level 01 with an enemy of level 03 in its set is saved and played
- **THEN** that enemy looks as in level 03
