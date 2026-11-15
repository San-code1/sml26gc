# Architecture

A Cargo workspace with a hexagonal architecture: pure domain crates in the middle, programs
with ports and adapters around them. Behaviour is guarded by the golden fingerprint in
`regression/`.

## Crates and dependencies

| Folder | Role |
|---|---|
| `crates/gb-core` | Domain: the Game Boy (CPU, bus, PPU, APU, cartridge MBCs). No I/O, no dependencies |
| `crates/cart-model` | Domain: what is known about this cartridge (levels, objects, world map, patches, RAM map) and use cases on a running console |
| `crates/host` | Shared adapters: finding the cartridge, file locations, battery saves, settings file, PNG output |
| `game/` | Application: the game (`game.exe`) |
| `editor/` | Application: the editor (`editor.exe`) |
| `tools/research/` | Reverse-engineering CLI (`research.exe`) |
| `regression/` | Golden fingerprint test |
| `legacy/gb_port/` | The original single-crate port, kept as reference; built on its own |

```
 game ─────┐                     tools/research ─┐
 editor ───┼──► host ──► cart-model ──► gb-core ◄─┘
           └──────────────►┘
```

- `gb-core` knows nothing about the game, files, macroquad or cpal.
- `cart-model` works on `&[u8]` ROM images and `GameBoy`; no files, no UI.
- `host` knows nothing about macroquad or cpal.
- `game` and `editor` do not depend on each other; they share only the contract below.

## `gb-core`

Everything goes through `GameBoy<O: BusObserver>` (alias `Gb`): `run_frame`, `set_buttons`,
`read`/`write` through the bus, `screen`, `audio`, `video`, `cart`. `Buttons` is a typed set.

- `BusObserver` is generic (static dispatch); `NoObserver` compiles away. Research tools use it
  to watch memory accesses.
- No `dyn` in the CPU loop or memory accesses; the MBC is an `enum` (a closed set of hardware).

## `cart-model`

Each module documents the ROM addresses and formats it relies on.

| Module | Contents |
|---|---|
| `rom` | `offset(bank, addr)`, `word`, `set_word` |
| `ram_map` | Named RAM addresses and `GameMode` |
| `levels` | Level headers, map locations, map compression (`decode`/`encode`) |
| `objects` | Object lists: 3-byte format, reading, re-packing |
| `patch` | The 1 MB edited copy (twin banks $20-$3F) that gives levels and objects more room |
| `layout` | Each zone's space for its maps and packing edited maps back |
| `extra_levels` | Levels 32+, stored in banks $38-$3F and played in place of a base level |
| `world` | World map picture and the level each map point opens |
| `zone_maps` | The zones' one-screen map pictures |
| `original_points` | Where the game draws its map points (measured) |
| `map_routes` | Paths between map points (walk scripts), where Mario stands on each, new points; moved to bank $36 in the edited copy |
| `doors` | Which world points enter which zone and where each zone exit leads |
| `art` | Pictures captured from the running game (blocks, map tiles, object looks) |
| `live` | What the running game has loaded |
| `driver` | `GameDriver`: boot to the map, enter a level, back to the map, show a map point |
| `error` | `CartError` and what a save used |

## `game/`

```
game/src/
├─ domain/        keys, settings (text format), i18n, overlay (bitmap font), pause menu
├─ application/   preferences (settings + saving), session (one frame of play; new levels)
├─ ports.rs       Keyboard, AudioOut, Window, SettingsStore, SaveStore
├─ adapters/      macroquad keyboard/display/picker, cpal audio, file stores, Win32
├─ checks.rs, selftest.rs   headless checks (--research, --self-test, --dump)
└─ main.rs        composition root
```

`domain/` and `application/` use no macroquad, cpal, `host` or `std::fs`; tests replace the
ports with fakes.

## `editor/`

```
editor/src/
├─ domain/        document (everything being edited, dirty flags, commit to the ROM image),
│                 history (generic undo/redo), tools (Tool trait: pencil, box, select, paste),
│                 stamp (a rectangle of cells: brush and clipboard), prefs (editor.cfg text),
│                 files (level and project files: text format)
├─ application/   save (commit + write through a port), art_service (captured graphics),
│                 exchange (level and project files through the dialogs)
├─ ports.rs       EditedCopyStore, GameLauncher, PrefsStore, FileDialogs, TextFiles, MenuBar
├─ adapters/      file repository (edited copy, editor.cfg, text files), Win32 file dialogs
│                 and menu bar, game.exe launcher
├─ ui/            macroquad panels: level, objects, palettes, world map, status bar, widgets;
│                 grid_edit (painting and clipboard shared by both views), shortcuts (one
│                 table for keys and button labels), menu (the File menu), files_view
│                 (file actions and their messages)
└─ main.rs        composition root
```

The world map editor has two tabs: Picture (paint the world and zone maps) and Points (the
level each point opens, paths, new points, doors).

## `tools/research/`

A table of commands (`COMMANDS` in `main.rs`) over the cartridge: disassembly, memory watch and
trace, ROM views, and checks that play the edited data in the emulator (`routecheck`,
`doorcheck`...). `all_commands.sh` runs a fixed set and saves the output to compare builds.

## Contract between game and editor

| Item | Format |
|---|---|
| Start the game | `game.exe <rom> [--level N]` (no rom: start screen to choose one) |
| Edited copy | `edits/<name> (edit).gb` |
| Battery saves | `saves/<name>.sav` |
| Last cartridge chosen | `last_cartridge.txt` |
| Settings (the editor reads the language) | `settings.cfg` |
| Editor settings (palette zoom) | `editor.cfg` |
| Level and project files (editor only) | `exports/*.nmblevel`, `exports/*.nmbproject` |

Paths and names live in `host::paths`, so both programs compute them the same way.

## Tests

1. Golden fingerprint (`regression/`): must match `golden.txt`; an intended behaviour change
   updates the file on purpose.
2. Unit tests of the pure domain (codecs, history, tools, menu, routes, doors...); tests that
   need the cartridge skip themselves without it.
3. Research checks that play edits in the emulator.
