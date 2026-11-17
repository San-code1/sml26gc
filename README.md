# New Mario Bros

A Game Boy emulator in Rust that runs **your own** Super Mario Land 2 cartridge file, plus an
editor that changes the game's levels, objects and maps and writes an edited copy. For private
use: cartridges, saves and edited copies never go into this repository (see `.gitignore`).

Architecture: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). Contributing rules: [`AGENTS.md`](AGENTS.md).

## Build and run

    cargo build --release
    target\release\game.exe ["path\to\game.gb"] [--level N] [--cheats]
    target\release\editor.exe
    target\release\research.exe disasm 0 0361 20

The cartridge is found from the command line, a `rom` folder next to (or above) the program,
or `%USERPROFILE%\Games`. Saves (`saves\`), edited copies (`edits\`) and `settings.cfg` live
next to the executables.

## Game

| Game Boy | Keyboard (default) |
|---|---|
| D-pad | Arrows |
| A | X |
| B | Z |
| Start | Enter |
| Select | Backspace |

Esc (or Enter in a level) opens the pause menu: controls, display, sound, back to map, file
select, language. F11: full screen. Hold Ctrl: fast-forward.

The start screen's "Debug / cheats" box (C) adds a "Debug / cheats" entry to the pause menu;
the choice is kept in `settings.cfg` (`cheats=1`). The game started from the editor (F5,
`--cheats`) always has it. The cheats only change the game being played, never the file:

| Cheat | What it does |
|---|---|
| Invincible | On/off: enemies and spikes do not hurt Mario (falling off still does) |
| Finish level | Ends the level as its goal with the bell rung: bonus game, then the map |
| Secret exit | Ends the level through its secret exit, to where it leads (only in levels that have one) |
| Go to level | Any level of the copy (new ones by name), entered from its own map point |
| Summon item | Star, mushroom, fire flower, carrot or heart (a life) in front of Mario |
| Freeze timer | On/off: the level's timer does not go down |
| 99 lives, 999 coins | Sets the counter |
| 6 golden coins | Marks the six bosses beaten, so Wario's castle door opens (their levels close, as in the game) |
| Open the whole map | Marks every level cleared (secret exits taken too): every path opens; bosses count as beaten |
| Info panel | On/off: level, Mario's x and y, map point, game mode and active objects (hexadecimal) at the top left |
| Objects view | On/off: each active object's type over it |

Finish level, Secret exit and Summon item only work in a level. With the cheats mode on, F6
saves the whole game and F7 brings it back (in memory, until the game is closed), and holding
R plays the last 10 seconds backwards; these keys are set in Controls with the buttons, and
the tag shown when a level begins lists them with the key that opens the menu.
`--level N --start X,Y` starts level N with Mario at X (left) and Y (feet), this time only.

## Editor

- Levels: paint blocks, place and move objects (Tab), add new levels (Ctrl+N copies the level shown, Ctrl+B makes a blank one) and name them.
  PgUp / PgDn change level, F2 opens the world map editor. A game level or map whose saved
  copy differs from the game's (blocks, objects, start, points, picture) is marked "mod".
- Where Mario begins a level is drawn on it; Place start (I) puts him on the block clicked, and
  he can be dragged (the game's camera follows from his place). Place finish (F) moves where
  the level ends (its goal block; the bell above it goes along); levels that end with their
  boss have none. Secret exit (Shift+F) places a level's secret exit block, and Leads to (U)
  chooses on the maps where it takes Mario (a world point, a zone's point or a new zone's);
  until then a game level's goes where the game's does.
- Checkpoint (K) adds a midway bell at the cell clicked; after ringing it and losing a life
  Mario starts again just left of it on the ground below (drawn as his outline). Up to 4 per
  level (dragged and deleted as objects); level 31 cannot have them. With more than one the
  copy gets a hook that remembers which bell was rung last.
- The panel sets the level's start time (100-900), music (Listen plays it with the game's own
  sound, at the listening volume kept in `editor.cfg`), background shades, gravity (normal or
  space), auto-scroll (the screen moves right by itself; Mario caught against a wall loses a
  life), camera margin (wide: the camera moves up sooner) and, where the level has them,
  animated tiles (its own or none), for the game's levels and new ones, and lists its warnings (no finish, a bell without its finish,
  more than 6 objects in one screen, more than 83 objects, objects on the screen the level
  begins on (the game brings objects to life only at the screen's edge, so those never
  appear; Objects mode frames that screen), Mario's start or a restart inside
  blocks, a map point opening a level that does not exist); a click shows the place, and each
  level button shows its count.
- World map: paint the world and zone maps (Picture tab); choose each point's level, draw
  paths, add points (46 new on the world map; up to 40 in all on each zone but the bonus
  game) and set doors and exits (Points tab). Where a door or exit leads is chosen on the maps
  (U): a world point, or the point a zone is entered on, which becomes that zone's entry.
  Points are dragged on every map (their paths become straight); the tab shows how many
  points the map has and can have. Points added from number $80 work only in this game, which
  keeps their progress in `saves\<game>.zones`; they cannot be where a door, exit or secret
  exit leads. Each of their paths can be shut until the point's level is cleared (Ctrl+arrow
  keys). A path drawn from a point that stands on another path's road replaces that path's
  way back, so Mario stops at the new point.
- The game's own points are drawn as Mario meets them, and the Points tab says what the
  original game does with the chosen one besides its level and paths (the flight to the Space
  Zone after clearing point $05's level, a zone entrance and the number Mario stands on there
  after leaving the zone, whose level follows it, points Mario never stands on) and whether it
  was changed. The panel scrolls with the wheel.
- New zones (Ctrl+N on a zone map, Ctrl+Del deletes): a copy of the Tree, Pumpkin, Macro,
  Space or Mario zone with its own name, picture, points (up to 40), paths, levels,
  exits and progress, entered from any world point. It plays with its base zone's graphics
  and music, and only in this game: the game puts it in place of its base while Mario is at
  its door or inside, and keeps its progress in `saves\<game>.zones`.
- New blank level (Ctrl+B, the File menu or Blank level): choose one of the game's 32 levels
  seeing only its blocks and the objects it draws right; the new level plays as it (tiles,
  objects, music, time) and starts as an empty sky with a floor and Mario's start.
- The object palette offers the types the level draws right (its own, and items that look
  the same there), filtered by category with its < > buttons; Show all adds the others,
  framed, as they may look broken. It names the chosen type and the one under the mouse as
  the object catalogue does. Choosing a type lets the selected object go (Shift+click changes
  the selected object to that type instead). Clicking the
  chosen type again, or right-clicking an empty place, leaves none chosen (clicks then only
  pick objects; a right click on an object deletes it).
- The object catalogue (F3, or Catalogue in the Objects palette) lists the game's objects once
  each with a name, picture, category and the levels that use them; Use picks one's type. Each
  cell's tag says what it is to the level shown: none (the level draws it), added, fits, no
  room, not here (other graphics) or other catalogue. Two filters combine: Show (all,
  compatible, usable here or can be added; one at a time, as each is part of compatible) and
  category. The palette shows a type's name
  when the mouse is over it. An enemy, trap or platform of another level of the same catalogue
  (levels 0-9 or 10-31) that draws with enemy tiles can be added to the level's own enemy set
  when its tiles fit in the level's 56 enemy tiles (each cell says whether it fits before Add,
  and which object it clashes with). Objects added show "added" in the catalogue and count as
  usable here. Remove from this level in the catalogue takes any object of the set out (the
  level's own too, to make room), with every object of that type, as they would look broken;
  one Ctrl+Z brings them back. In the palette the objects brought from other levels have a
  corner mark; Remove beside the chosen one or a right click on it takes it out the same way.
  The Tools menu opens the catalogue too. Works for the game's levels and new ones; the copy
  keeps the sets' sprite blocks where the original maps were.
- Messages say their kind: done in green (+), warnings in orange (!), errors in red (x), the
  rest as plain information.
- Restore (Ctrl+Shift+R) puts back the level or map shown as in the game; a new level or zone,
  as its base.
- Painting: Pencil (B) and Box (R) paint the palette selection; drag over the palette, or
  right-drag on the map, to take several cells at once. Select (E), then Ctrl+C / Ctrl+X /
  Del, and Ctrl+V to paste where it was copied (Esc stops).
- Palettes zoom with their - + buttons or Ctrl - / Ctrl +; the side panel widens or narrows to
  fit (at most half the window). A cut palette scrolls with the wheel or a right-drag. The zoom
  is kept in `editor.cfg`.
- Files, from the File menu or the keys (Windows Save and Open dialogs, starting in
  `exports\`): Ctrl+E exports the level
  shown; Ctrl+Shift+N imports a level file as a new level, Ctrl+I replaces the new level shown
  with one (the game's 32 levels are never replaced, only edited). On a zone map the same keys
  export the zone and import or replace a new zone. Ctrl+Shift+E / Ctrl+Shift+I export and
  import the whole project (every level, the maps, points, paths, doors and new zones) over the
  same cartridge; nothing is written until you save.
- The level editor's panel: the level picker (its grid unfolds with the middle button), a
  toolbar, Blocks / Objects with their tools, and two tabs: Palette, and Level (its start,
  goals, checkpoints, settings and warnings; it scrolls with the wheel). Its buttons show
  their keys in a tooltip; the world map editor's show them in the label.
- Every button has a shortcut. Ctrl+Z / Ctrl+Y undo and redo (in the order the changes were
  made, whatever the mode or tab: blocks, objects and goals; map pictures and points), Ctrl+S saves
  `edits\<game> (edit).gb` (the original is never changed), F5 saves and plays it (with the cheats on);
  Shift+F5 (Here), then a click on the level, plays it with Mario on the clicked block.
- Object pictures are captured from the game in the background (the shown level's first), then
  the tiles each of the game's objects shows, and kept in `cache\` next to the editor, so later
  starts have them at once. Until it ends a box at the bottom right shows how far it is, the
  palette, catalogue and blank level chooser say they are not complete yet, and adding or
  removing set enemies waits for their tiles. Tools, Capture the object pictures again forgets
  them and captures them all again.

## Checks

    cargo test --release --workspace        # includes the golden fingerprint
    target\release\game.exe --self-test out.json
    target\release\game.exe --research menushots DIR
