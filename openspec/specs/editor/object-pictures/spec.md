# editor/object-pictures Specification

## Purpose
How the level editor gets the game's own pictures of the objects without making the person
wait, and how its marks over the game's pictures stay visible.

## Requirements

### Requirement: Object pictures without waiting
The editor SHALL capture the objects' pictures from the game in the background from the
moment it opens, the shown level's objects first, and SHALL keep the window responsive while
it does. Captured pictures SHALL be kept between starts for each cartridge, so a later start
shows them at once. Objects whose picture is not captured yet SHALL be drawn as a numbered
box until it is.

#### Scenario: Objects tab right after opening
- **WHEN** the editor has just opened and the Objects tab is chosen
- **THEN** the window keeps responding, and the level's objects get their pictures as they come

#### Scenario: Next start
- **WHEN** the editor is opened again with the same cartridge
- **THEN** the objects already captured are drawn with their pictures at once

### Requirement: Typing is only typing
While a name is being typed (a new level's or zone's), the keys SHALL go only to the name: the
view SHALL NOT move with the arrows, WASD or Space, and no shortcut SHALL run.

#### Scenario: A name with WASD
- **WHEN** "wasd" is typed into a new level's name
- **THEN** the name reads "wasd" and the view stays where it was

### Requirement: Marks that stand out
The editor's marks over the game's pictures (an object's outline and type, the start, the
finish, the secret exit, the checkpoints, map points and the chosen point) SHALL be drawn in
colours none of the game's four shades is near, over a dark border, so they can be seen on any
block or tile.

#### Scenario: An object on a dark block
- **WHEN** an object stands on the darkest blocks of a level
- **THEN** its outline and type are as easy to see as on the lightest ones
