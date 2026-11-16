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
