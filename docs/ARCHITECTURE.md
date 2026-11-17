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
| `level_goals` | Where a level ends: the finish block $4A and the secret exit block $4B in its map, the bell (object $16) above the finish, the boss levels that have none |
| `secret_exits` | Where a level's secret exit takes Mario: a hook at 0F:6F62 and its table at 0F:7E40 (level to target point; a new level takes its base's entry while played) |
| `level_start` | Where Mario begins a level: header bytes $00-$07 (his y and x, the camera's), the camera worked out from his place |
| `checkpoint` | Midway bells (object $17) and their restart places (0:377B); up to 4 per level through a hook (touch, bell, map flag bits 4-5, restart table in bank $21) |
| `level_settings` | Header bytes of a level's start time ($13), music ($0E) and background shades ($0F) |
| `level_tables` | A level's entries in the bank-0 tables of gravity, auto-scroll, camera margin and animated tiles |
| `level_graphics` | Which levels draw the same objects (enemy sprites 0:362A and the object catalogue split at level 10) |
| `enemy_sets` | A level's enemies chosen from several levels: the sprite block built for them, where the copy keeps it (the low copies of the map banks' maps) and the 0:362A entry |
| `mixed_catalogues` | The bytes a copy saved with the dropped object catalogue mixing patch has changed, and giving the game's back |
| `blank_level` | An empty map and start made from a game level's (its sky, a floor under Mario's start) |
| `objects` | Object lists: 3-byte format, reading, re-packing |
| `patch` | The 1 MB edited copy (twin banks $20-$3F) that gives levels and objects more room |
| `layout` | Each zone's space for its maps and packing edited maps back |
| `extra_levels` | Levels 32+, stored in banks $38-$3F and played in place of a base level |
| `new_zones` | New zones, stored in bank $31 and played in place of a base zone (point numbers, entry code, progress kept apart) |
| `world` | World map picture and the level each map point opens |
| `zone_maps` | The zones' one-screen map pictures |
| `original_notes` | What the original game does with some map points (Space Zone flight, entrance twins, unused door) |
| `original_points` | Where the game draws its map points (measured) |
| `map_routes` | Paths between map points (walk scripts), where Mario stands on each, new points; moved to bank $36 in the edited copy, room for 256 numbers |
| `added_points` | Points numbered from $80 on every map: hooks in bank $0F (level, flag, map pick), their flag bytes at $A1A0, the game zones' kept in bank $24 |
| `doors` | Which world points enter which zone and where each zone exit leads |
| `cheats` | What each cheat does in the running game: the invincible patch (0:2E95, 0:1C58), a summoned item in a free object slot, finishing the level through the bell and goal or its secret exit, going to a level, opening the whole map, the active objects, the timer, lives, coins and golden coins, the info panel's values |
| `art` | Pictures captured from the running game (blocks, map tiles, object looks) |
| `live` | What the running game has loaded |
| `driver` | `GameDriver`: boot to the map, enter a level, back to the map, show a map point |
| `error` | `CartError` and what a save used |

## `game/`

```
game/src/
├─ domain/        keys, settings (text format), i18n, overlay (bitmap font), pause menu,
│                 cheats (which there are, the lasting ones, the info and object tags)
├─ application/   preferences (settings + saving), session (one frame of play; new levels;
│                 cheats, save states and rewind), zones (new zones swapped in and out,
│                 their progress)
├─ ports.rs       Keyboard, AudioOut, Window, SettingsStore, SaveStore, ZoneProgressStore
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
│                 files (level, zone and project files: text format); document/zones
│                 (new zones), document/restore and document/check (level warnings),
│                 look_cache (captured object pictures and tiles as text), object_catalogue
│                 (the game's objects once each), object_names (the editor's names for them)
├─ application/   save (commit + write through a port), art_service (captured graphics),
│                 exchange (level, zone and project files through the dialogs), music
│                 (a level's music played by a console of the copy), look_loader (object
│                 pictures and tiles captured on background threads, one per spare core)
├─ ports.rs       EditedCopyStore, GameLauncher, PrefsStore, FileDialogs, TextFiles, MenuBar,
│                 SoundOut, LookStore
├─ adapters/      file repository (edited copy, editor.cfg, text files), Win32 file dialogs
│                 and menu bar, game.exe launcher, cpal speakers
├─ ui/            macroquad panels: level, objects, palettes, world map, status bar, widgets;
│                 grid_edit (painting and clipboard shared by both views), shortcuts (one
│                 table for keys and button labels), menu (the File menu), files_view
│                 (file actions and their messages), names_view (naming new levels and
│                 zones), target_picker (where a door or exit leads, chosen on the maps),
│                 level_panel (the level side panel: picker, toolbar, tabs, inspector),
│                 testing_view (checkpoints and play here drawn on the level)
└─ main.rs        composition root
```

The world map editor has two tabs: Picture (paint the world, zone and new zone maps) and
Points (the level each point opens, paths, new points, doors, exits, entries).

## `tools/research/`

A table of commands (`COMMANDS` in `main.rs`) over the cartridge: disassembly, memory watch and
trace, ROM views, and checks that play the edited data in the emulator (`routecheck`,
`doorcheck`, `newzonecheck`, `sparecheck`...). `all_commands.sh` runs a fixed set and saves the output to compare builds.

## Contract between game and editor

| Item | Format |
|---|---|
| Start the game | `game.exe <rom> [--level N [--start X,Y]] [--cheats]` (no rom: start screen to choose one; `--start`: Mario's left x and feet y for that start only) |
| Edited copy | `edits/<name> (edit).gb` |
| Battery saves | `saves/<name>.sav` |
| New zone and added point progress (game only) | `saves/<name>.zones` |
| Last cartridge chosen | `last_cartridge.txt` |
| Settings (the editor reads the language) | `settings.cfg` |
| Editor settings (palette zoom, listening volume) | `editor.cfg` |
| Captured object pictures (editor only) | `cache/looks-<hash of the cartridge>.txt` |
| Level, zone and project files (editor only) | `exports/*.nmblevel`, `exports/*.nmbzone`, `exports/*.nmbproject` |

Paths and names live in `host::paths`, so both programs compute them the same way.

## Tests

1. Golden fingerprint (`regression/`): must match `golden.txt`; an intended behaviour change
   updates the file on purpose.
2. Unit tests of the pure domain (codecs, history, tools, menu, routes, doors...); tests that
   need the cartridge skip themselves without it.
3. Research checks that play edits in the emulator.
