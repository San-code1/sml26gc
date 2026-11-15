# New Mario Bros

A Game Boy emulator in Rust that runs **your own** Super Mario Land 2 cartridge file, plus an
editor that changes the game's levels, objects and maps and writes an edited copy. For private
use: cartridges, saves and edited copies never go into this repository (see `.gitignore`).

Architecture: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). Contributing rules: [`AGENTS.md`](AGENTS.md).

## Build and run

    cargo build --release
    target\release\game.exe ["path\to\game.gb"] [--level N]
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

## Editor

- Levels: paint blocks (Pencil B, Box R, Grid G), place and move objects (Tab), add new levels.
- World map: paint the world and zone maps (Picture tab); choose each point's level, draw
  paths, add points and set doors between the world and the zones (Points tab).
- Ctrl+Z / Ctrl+Y undo and redo, Ctrl+S saves `edits\<game> (edit).gb` (the original is never
  changed), F5 saves and plays it.

## Checks

    cargo test --release --workspace        # includes the golden fingerprint
    target\release\game.exe --self-test out.json
    target\release\game.exe --research menushots DIR
