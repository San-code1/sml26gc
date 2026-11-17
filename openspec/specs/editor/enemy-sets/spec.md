# editor/enemy-sets Specification

## Purpose
A level's enemies, traps and platforms chosen among the game's, from several levels of its
object catalogue, as long as they fit together in the level's enemy sprite area and the level
loads their common graphics, drawn by the game as in their own levels.

## Requirements

### Requirement: Enemies that fit together
For any level, new or one of the game's, the editor SHALL let the user choose its enemy set
(starting from the level's own enemies): enemies of the level's object catalogue whose sprite
tiles in the enemy area do not overlap, added from the object catalogue and removed from it
(Remove from this level, the level's own enemies too). An enemy that does not fit SHALL not be
added, and the editor SHALL name the enemy it clashes with.

#### Scenario: Two that fit
- **WHEN** level 01's frog is removed from its set in the catalogue and level 00's Koopa Troopa is added
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
say which level when the mouse is over one. Remove beside the chosen object in the palette, a
right click on it in the palette, or Remove from this level in the catalogue (for any object of
the set), SHALL take it out of the set and every object of its type out of the level (they
would look broken), in one undo step. The Level tab SHALL have no enemies section.

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

### Requirement: Traps and platforms in a set
A level's set SHALL take traps and platforms of other levels of its object catalogue like
enemies: added from the catalogue when their enemy-area tiles fit, marked in the palette and
removed with their objects. A platform drawn without enemy-area tiles SHALL be offered where
the level draws it right, without a set.

#### Scenario: A falling spike from another level
- **WHEN** level 06's falling spike is chosen in the catalogue while level 03 is shown
- **THEN** the panel offers Add (or says which object of the set its tiles clash with)

### Requirement: Every tile an object shows
The tiles a set copies for an object SHALL be every tile it shows, whether Mario stands still,
walks, jumps on it or touches it, not only those of the first frames.

#### Scenario: An enemy that animates when Mario comes
- **WHEN** an enemy whose moving frames use other tiles is added to a level's set and played
- **THEN** every frame is drawn whole, as in its own level

### Requirement: Common graphics
An object that also draws with sprite tiles outside the enemy area SHALL be offered for a set
only where the level loads the same graphics into those tiles as the object's own level; the
catalogue SHALL say why otherwise.

#### Scenario: Other common graphics
- **WHEN** an object drawing with common tiles is chosen while a level that loads other common graphics is shown
- **THEN** it cannot be added and the panel says the level loads other graphics there

### Requirement: Only the level's own catalogue
A level's set SHALL hold only objects of its own object catalogue; a copy whose sets hold
others SHALL have them, and their objects, taken out when opened (each an undo step), with a
message.

#### Scenario: A set saved with other-catalogue members
- **WHEN** a copy whose level 33 (made from level 00) has level 12's Bullet Bill in its set is opened
- **THEN** the Bullet Bill and its objects are taken out of level 33, and the message says so
