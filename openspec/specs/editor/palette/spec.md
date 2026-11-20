# editor/palette Specification

## Purpose

The editor's palettes (level blocks, object types, world and zone map tiles) let the user choose
what to paint, at a size that suits their screen, one cell or a rectangle of cells at a time;
the object palette offers the types the level draws right, named as the object catalogue
names them and filtered by category; choosing a type never changes the object just placed.

## Requirements

### Requirement: Palette zoom
The block palette, the object type palette and the map tile palette SHALL each be shown at a
user-chosen zoom, changed with zoom in/out buttons beside the palette title and with keyboard
shortcuts. The zoom SHALL have at least four steps, the smallest at most the current size.

#### Scenario: Zoom in on the tile palette
- **WHEN** the world map editor's Picture tab is shown and the user presses the palette zoom in button
- **THEN** every tile in the palette is drawn larger and clicking a tile still chooses that tile

#### Scenario: Zoom limits
- **WHEN** the palette is at its largest zoom and the user zooms in again
- **THEN** the zoom stays the same

### Requirement: Palette scrolling
A palette larger than the space the panel has for it SHALL scroll with the mouse wheel over it
(Shift+wheel horizontally) and SHALL keep its layout (the same cells per row at every zoom).

#### Scenario: Scroll a zoomed palette
- **WHEN** the block palette is taller than the panel and the user turns the wheel over it
- **THEN** the palette scrolls and the map view's zoom does not change

#### Scenario: Layout kept
- **WHEN** the tile palette is zoomed in
- **THEN** it still shows 16 tiles per row, in tile number order

### Requirement: Remembered palette zoom
The palette zoom SHALL be kept when the editor closes and restored when it opens, in an editor
settings file next to the editor, separately for the level palettes and the map tile palette.
A missing or unreadable file SHALL give the default zoom.

#### Scenario: Zoom restored
- **WHEN** the user sets the tile palette to its largest zoom, closes the editor and opens it again
- **THEN** the tile palette is shown at its largest zoom

#### Scenario: No settings file
- **WHEN** the editor opens and there is no editor settings file
- **THEN** the palettes are shown at the default zoom and nothing fails

### Requirement: Multi-cell palette selection
Dragging over the block palette or the tile palette SHALL select the rectangle of cells
between where the drag started and ended (a stamp); a click SHALL select one cell. The
selection SHALL be outlined in the palette and the panel title SHALL show its size.

#### Scenario: Select a 2x2 stamp
- **WHEN** the user drags from tile $20 to tile $31 in the tile palette (16 per row)
- **THEN** tiles $20, $21, $30 and $31 are selected as a 2 by 2 stamp

#### Scenario: Single click
- **WHEN** the user clicks block $05 in the block palette
- **THEN** the stamp is block $05 alone, as before this change

### Requirement: Side panel fitted to the palette
The side panel's width SHALL follow the zoom of the palette it shows so that the palette's whole
width fits: at least 300 px (its buttons are never cut) and at most half the window. It SHALL
not be resizable by hand. In the world map editor the tile palette's width SHALL be used in
both tabs.

#### Scenario: Zoom in widens the panel
- **WHEN** the tile palette is at zoom 2 and the user zooms in
- **THEN** the panel widens so the 16 tile columns at zoom 3 fit, and the map view starts further right

#### Scenario: Zoom out never cuts the buttons
- **WHEN** the block palette is at its smallest zoom
- **THEN** the panel is 300 px wide and every button is whole

#### Scenario: Too big for half the window
- **WHEN** the palette at the chosen zoom is wider than half the window
- **THEN** the panel is half the window wide and the palette scrolls

### Requirement: Dragging a palette
Holding the right mouse button over a palette and moving the mouse SHALL scroll it in every
direction, the palette following the mouse.

#### Scenario: Drag to hidden tiles
- **WHEN** the tile palette is cut on the right and the user right-drags it to the left
- **THEN** the hidden columns come into view and no tile is chosen

### Requirement: Palette scroll bars
When part of a palette is hidden, a thin bar along its bottom (columns hidden) or its right side
(rows hidden) SHALL show which part is in view.

#### Scenario: Bars shown only when needed
- **WHEN** the whole palette fits
- **THEN** no scroll bar is drawn

THEN** the zoom becomes the largest at which 16 tiles fit across the panel

### Requirement: Objects the level draws right
The object palette SHALL offer only the types the shown level draws right: the bells, and the
types a game level of the same object catalogue uses that loads the same graphics for them or
draws them with the same pixels as the shown level. A Show all switch SHALL add the other
types, marked as possibly broken. Objects already in the level SHALL stay as they are.

#### Scenario: Level 01's objects
- **WHEN** the object palette of level 01 is shown
- **THEN** it offers level 01's enemies and the items that look right there, and not the Space Zone's enemies

#### Scenario: Show all
- **WHEN** Show all is turned on
- **THEN** every type is offered and those level 01 may draw broken are marked

### Requirement: No object type chosen
Choosing in the object palette the type already chosen, or a right click on the level where
there is no object, SHALL leave no type chosen and no object selected; a click on an empty
place then places nothing. A right click on an object SHALL still delete it.

#### Scenario: Put the type down
- **WHEN** the mushroom is chosen in the object palette and is clicked again
- **THEN** no type is chosen and a click on an empty place of the level adds no object

#### Scenario: Right click on the sky
- **WHEN** a type is chosen and the user right-clicks where there is no object
- **THEN** no type is chosen and no object is deleted

### Requirement: Object names in the palette
The object palette SHALL show the chosen type's name and category, and the type under the
mouse's, as the object catalogue names the object drawn there (its number alone when the
catalogue has none), so the palette and the catalogue never disagree.

#### Scenario: Pointing at the mushroom
- **WHEN** the mouse is over the mushroom in level 01's object palette
- **THEN** the palette shows its name and that it is a power-up, as the catalogue does

### Requirement: Palette category filter
The object palette SHALL have a category filter (all objects, or one of the catalogue's
categories), stepped with its < and > buttons, that shows only the types of that category,
together with Show all.

#### Scenario: Only enemies
- **WHEN** the filter is stepped to Enemies in level 01
- **THEN** the palette shows level 01's enemies only

### Requirement: Choosing a type keeps the placed object
Choosing a type in the object palette SHALL let the selected object go, unchanged; Shift+click
on a type SHALL change the selected object to that type instead, and the palette SHALL say so
while an object is selected.

#### Scenario: Placing two different objects
- **WHEN** a Goomba is placed (and stays selected) and the Koopa is then clicked in the palette
- **THEN** the Goomba stays a Goomba, nothing is selected, and the next click places a Koopa

#### Scenario: Changing the selected object
- **WHEN** an object is selected and a type is Shift+clicked in the palette
- **THEN** the selected object becomes that type

### Requirement: Nothing chosen until the user chooses
The block, tile and object palettes SHALL have nothing chosen when a project is opened and whenever another level or map is shown; painting on the level or map SHALL do nothing until the user chooses a block or tile (in the palette or by picking one from the grid), and clicking the level SHALL add no object until the user chooses a type.

#### Scenario: Opening a project
- **WHEN** the user opens a project and clicks on the level with the Pencil
- **THEN** no block is painted, the palette outlines no block and its title says nothing is chosen

#### Scenario: Another level or map
- **WHEN** block $12 is chosen and the user shows another level, or another map in the zone editor
- **THEN** no block or tile is chosen there until the user picks one

### Requirement: The chosen cell stands out
The chosen block, tile or object type SHALL be outlined in the palette with a bright mark colour framed in black, visible over light and dark cells alike.

#### Scenario: A dark block chosen
- **WHEN** the user chooses a dark block in the block palette
- **THEN** its outline is bright and framed in black, plainly visible over it and the cells around

### Requirement: Blocks and Objects show the palette
In the level editor, choosing Blocks or Objects (button or key) SHALL show the Palette tab.

#### Scenario: From the Level tab
- **WHEN** the Level tab is shown and the user presses Objects
- **THEN** the Palette tab is shown with the object palette

### Requirement: Recently chosen
The block, tile and object palettes SHALL show, above their cells, the last 8 single blocks, tiles or object types chosen in the level or map shown, newest first; a click on one chooses it again. The row SHALL be empty until something is chosen there.

#### Scenario: Going back to a block
- **WHEN** the user chooses block $12, then $30, then clicks $12 in the recent row
- **THEN** $12 is chosen and the row shows $12 then $30

### Requirement: A zone's tiles as its picture
On a game zone or a new zone, the tile palette SHALL show the game's own picture of the zone (a new zone's base zone), 20 tiles per row as the game draws it, then under a rule the tiles that picture does not use; a click or a drag on it chooses those tiles as the brush. The world map's palette SHALL stay its 256 tiles in order.

#### Scenario: Taking the tree's trunk
- **WHEN** the Tree Zone is shown and the user drags over the trunk in the palette
- **THEN** the brush is those tiles as the picture has them, and painting puts the trunk down

#### Scenario: A tile the picture leaves out
- **WHEN** the user clicks a tile under the rule
- **THEN** that tile is the brush

### Requirement: The world map's pieces
On the world map, the tile palette SHALL show ready-made pieces of the game's own world map picture (the zones' landmarks, Wario's castle, the houses, the bridge, the ladder, the pipe), named by hand, side by side, then under a rule the tiles they leave out. The piece under the mouse SHALL be named in a tooltip; a drag SHALL stay inside the piece it starts in, so dragging past a piece takes all of it.

#### Scenario: Taking Wario's castle
- **WHEN** the user drags from the top left of Wario's castle in the palette to beyond its bottom right
- **THEN** the brush is the whole castle as the game draws it

#### Scenario: A loose tile
- **WHEN** the user clicks a tile under the rule
- **THEN** that tile is the brush

### Requirement: The palette title names the piece
When the brush was taken from a named piece of the map tile palette, the palette's title SHALL show the piece's name and the brush's size instead of the first tile's number.

#### Scenario: Wario's castle
- **WHEN** the user takes the whole of Wario's castle in the world map's palette
- **THEN** the title reads TILES  Wario's castle  14x8

### Requirement: Searching objects by name
The object palette SHALL have a search field under its category filter: the types shown are those whose name (in the editor's language) holds the text typed, or whose number it is (12, #12, $12). While it is typed in it takes the keyboard; Esc or its x button clears it.

#### Scenario: Looking for the bell
- **WHEN** the user types bell in the search field (campana in Spanish)
- **THEN** only the bells the level offers are shown
