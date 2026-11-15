# editor/palette Specification

## Purpose

The editor's palettes (level blocks, object types, world and zone map tiles) let the user choose
what to paint, at a size that suits their screen, one cell or a rectangle of cells at a time.

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
