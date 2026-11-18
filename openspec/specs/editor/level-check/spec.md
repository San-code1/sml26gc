# editor/level-check Specification

## Purpose
Warnings about mistakes in the shown level that would only show up when it is played,
listed in the level editor with the place each one is about.

## Requirements

### Requirement: Level warnings
The level editor SHALL check the shown level after each change and list its warnings in the
level panel, with a count on the level's button. Warnings SHALL NOT stop saving or playing.

#### Scenario: No finish
- **WHEN** the finish of a level that does not end with its boss is erased
- **THEN** the panel lists "No finish" and the level's button shows one warning

### Requirement: What is checked
The checks SHALL be: a level that does not end with its boss has no finish; a finish without
its bell (when the game's level it is played as has one: levels 25-31 have none), or a bell
with no finish below it; a screen (160x144 pixels) holding more objects
than the game shows at once; more objects in the level than the game holds; Mario's start or
the checkpoint's restart place inside solid blocks or outside the level; a map point that
opens a level that does not exist.

#### Scenario: A crowded screen
- **WHEN** more objects than the game shows at once are put in one screen
- **THEN** the panel lists the crowded screen

### Requirement: Going to a warning
Clicking a warning SHALL move the view to the place it is about and mark that place for a
moment; warnings that are not about one place (the object count) SHALL only be listed.

#### Scenario: Show the crowded screen
- **WHEN** the crowded screen warning is clicked
- **THEN** the view shows that screen with it marked

### Requirement: Objects on the first screen
The game brings an object to life when its x comes in past the screen's edge (33 pixels past
the left edge, 25 past the right edge), whatever its height, so the objects whose x lies in the
band from 33 pixels left of the screen the level begins on to 24 pixels right of it, over the
whole height of the level, do not appear when the level begins; they appear only once Mario has
gone far enough away and comes back. The level check SHALL warn about them (how many, showing
the first), and the level view SHALL frame that band in Objects mode, or in any mode while
objects are in it.

#### Scenario: A mushroom in front of Mario
- **WHEN** an object is placed 48 pixels right of Mario's start
- **THEN** the warnings list says one object does not appear at the start, and the band is framed in the warning colour

#### Scenario: Just past the screen's edge
- **WHEN** an object is placed 20 pixels past the right edge of the screen the level begins on, at any height
- **THEN** it is in the band and warned about

#### Scenario: Further on
- **WHEN** the object is moved 240 pixels right of Mario's start
- **THEN** the warning is gone and the object appears when the screen reaches it
