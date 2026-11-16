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
| Summon item | Star, mushroom, fire flower, carrot or heart (a life) in front of Mario |
| Freeze timer | On/off: the level's timer does not go down |
| 99 lives, 999 coins | Sets the counter |
| 6 golden coins | Marks the six bosses beaten, so Wario's castle door opens (their levels close, as in the game) |
| Info panel | On/off: level, Mario's x and y, map point and game mode (hexadecimal) at the top left |

Finish level, Secret exit and Summon item only work in a level.

## Editor

- Levels: paint blocks, place and move objects (Tab), add new levels (Ctrl+N) and name them.
  PgUp / PgDn change level, F2 opens the world map editor. A game level or map whose saved
  copy differs from the game's (blocks, objects, start, points, picture) is marked "mod".
- Where Mario begins a level is drawn on it; Place start (I) puts him on the block clicked, and
  he can be dragged (the game's camera follows from his place). Place finish (F) moves where
  the level ends (its goal block; the bell above it goes along); levels that end with their
  boss have none. Secret exit (Shift+F) places a level's secret exit block, and Leads to (U)
  chooses on the maps where it takes Mario (a world point, a zone's point or a new zone's);
  until then a game level's goes where the game's does.
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
- New zones (Ctrl+N on a zone map, Ctrl+Del deletes): a copy of the Tree, Pumpkin, Macro,
  Space or Mario zone with its own name, picture, points (up to 40), paths, levels,
  exits and progress, entered from any world point. It plays with its base zone's graphics
  and music, and only in this game: the game puts it in place of its base while Mario is at
  its door or inside, and keeps its progress in `saves\<game>.zones`.
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
- Every button shows its shortcut. Ctrl+Z / Ctrl+Y undo and redo, Ctrl+S saves
  `edits\<game> (edit).gb` (the original is never changed), F5 saves and plays it (with the cheats on).

## Checks

    cargo test --release --workspace        # includes the golden fingerprint
    target\release\game.exe --self-test out.json
    target\release\game.exe --research menushots DIR
