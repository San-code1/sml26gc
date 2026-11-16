# editor/object-catalogue Specification

## Purpose
The game's objects listed once each in the level editor, with a name, a picture, what kind
of object each is, where the game uses it and where it can be used, so a level can be built
knowing what each object is.

## Requirements

### Requirement: Every object once
The catalogue SHALL list every distinct object the game's 32 levels use, per object catalogue
(levels 0-9 and 10-31): the same object in several levels, or the same in both catalogues,
SHALL be listed once, with the type number it has in each catalogue.

#### Scenario: The mushroom
- **WHEN** the catalogue is opened
- **THEN** the mushroom is listed once, with the levels that use it

### Requirement: What each object is
Each listed object SHALL show a name, its picture, its category (power-up, enemy, trap,
platform, goal, invisible marker, boss or scripted) and the levels that use it. The names and
categories SHALL be the editor's own descriptions, not text from the cartridge.

#### Scenario: An enemy
- **WHEN** an enemy of level 01 is shown in the catalogue
- **THEN** it has a name, its picture, the category enemy and level 01 among its levels

### Requirement: Browsing the catalogue
The catalogue SHALL open from the Objects palette and with F3, filter by category and by
"usable in the level shown" (the objects the level draws right), and choosing an object
usable in the level shown SHALL choose its type in the object palette and close it.

#### Scenario: Pick a power-up
- **WHEN** the catalogue is opened in level 01, filtered to power-ups, and the mushroom is chosen
- **THEN** the catalogue closes and the object palette has the mushroom's type chosen

#### Scenario: Not usable here
- **WHEN** an object level 01 does not draw right is shown
- **THEN** it is marked as not usable in this level and cannot be chosen

#### Scenario: From the other object catalogue
- **WHEN** an enemy of levels 0-9 only is chosen in the catalogue while a level of 10-31 is shown
- **THEN** the panel says it belongs to the catalogue of levels 0-9 and why it cannot be used here, and offers a new blank level from levels 0-9, opening the chooser on a level that uses it

### Requirement: What waits for the capture
While object pictures and tiles are still being captured in the background, the editor SHALL
show how far it is (pictures and enemy tiles done and in all) in the level editor, the
catalogue and the blank level chooser; SHALL keep adding and removing enemies of a set
disabled, with the reason, until the tiles they need are captured; and SHALL say, in the
object palette, the catalogue and the blank level chooser, that what they offer is not
complete yet. Nothing else SHALL wait for it.

#### Scenario: Adding too soon
- **WHEN** an enemy of another level is chosen in the catalogue before the enemies' tiles are captured
- **THEN** its Add button is disabled and the panel says it will be available once they are, with how many are left

#### Scenario: Progress
- **WHEN** the editor is opened with an empty picture cache
- **THEN** a box at the bottom right counts the pictures and enemy tiles captured, and goes away when all are
