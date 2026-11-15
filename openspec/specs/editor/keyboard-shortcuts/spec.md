# editor/keyboard-shortcuts Specification

## Purpose

Every action of the editor's side panel can be done from the keyboard, and the panel shows
which key does it.

## Requirements

### Requirement: A shortcut for every panel action
Every action a panel button does SHALL also have a keyboard shortcut, active only in the view
and tab where the button is shown. Shortcut keys SHALL sit in the same place on US and Spanish
keyboards.

#### Scenario: Shortcut outside its tab
- **WHEN** the Picture tab is shown and the user presses N
- **THEN** no new point is placed

### Requirement: Existing keys kept
Keys in use before this change SHALL keep their meaning: arrows/WASD and Space pan, G grid,
B Pencil, R Box, M overview, Tab Blocks/Objects, Del/Backspace delete the selected object, F5
play, Ctrl+S save, Ctrl+Z undo, Ctrl+Y or Ctrl+Shift+Z redo.

#### Scenario: Pan keys
- **WHEN** the user holds D in any view
- **THEN** the view pans right and no action runs

### Requirement: Level and map navigation shortcuts
The editor SHALL provide these shortcuts:

| Action | Shortcut |
|---|---|
| Previous / next level (level editor); previous / next map (map editor) | PageUp / PageDown |
| New level (copy) | Ctrl+N |
| Delete level (new levels only) | Ctrl+Delete |
| World map editor / Back to the levels | F2 |
| Picture / Points tab | Tab |
| Original | Ctrl+Shift+O |

#### Scenario: New level from the keyboard
- **WHEN** the level editor is shown and the user presses Ctrl+N
- **THEN** a new level is made as with the New level (copy) button

#### Scenario: Next map
- **WHEN** the world map editor is shown and the user presses PageDown
- **THEN** the next map is shown as with the > button

### Requirement: Tool, palette and clipboard shortcuts
The editor SHALL provide these shortcuts in Blocks mode and the Picture tab:

| Action | Shortcut |
|---|---|
| Select tool | E |
| Palette zoom out / in | Ctrl+Minus / Ctrl+Plus |
| Copy / cut / paste | Ctrl+C / Ctrl+X / Ctrl+V |
| Fill the selection with the background | Delete |
| Clear the selection or leave Paste | Esc |

#### Scenario: Select tool key
- **WHEN** Blocks mode is shown and the user presses E
- **THEN** the Select tool is chosen

### Requirement: Points tab shortcuts
The editor SHALL provide these shortcuts in the Points tab:

| Action | Shortcut |
|---|---|
| Previous / next point | , / . |
| Level the point opens: previous / next | J / K |
| Door or Exit to: previous / next | U / I |
| Make it the exit (first / second exit of the zone) | T / Shift+T |
| Delete the path right, left, up, down | Shift+Right, Shift+Left, Shift+Up, Shift+Down |
| New path / New point / Delete point | P / N / Delete |

#### Scenario: Next point
- **WHEN** point $04 is chosen and the user presses .
- **THEN** point $05 is chosen, as with the Point + button

### Requirement: No shortcut does two things
Within one view and tab, each key combination SHALL trigger at most one action.

#### Scenario: Delete in Blocks and Objects mode
- **WHEN** a selection exists in Blocks mode and the user presses Delete
- **THEN** the selection is filled with the background and no object is deleted

### Requirement: Shortcuts shown on the buttons
Every button with a shortcut SHALL show it in its label (for example "Undo (Ctrl+Z)"); for the
small - and + buttons the keys SHALL be shown beside the row's label. A label too long for its
button SHALL be drawn smaller so it fits inside it.

#### Scenario: Undo label
- **WHEN** the level editor panel is shown
- **THEN** the Undo button reads "Undo (Ctrl+Z)" (Spanish: "Deshacer (Ctrl+Z)") and the text stays inside the button

#### Scenario: Point row
- **WHEN** the Points tab is shown
- **THEN** the Point row shows ", ." next to its - and + buttons
