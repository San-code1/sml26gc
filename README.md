# New Mario Bros (workspace)

Game Boy emulator in Rust that runs **your own** cartridge file, plus a level/object/world map
editor that writes an edited copy of it. For private use on your own computer: the cartridge,
saves and edited copies are never part of this repository (see `.gitignore`).

| Folder | What it is |
|---|---|
| `crates/gb-core` | The Game Boy (CPU, memory, picture, sound, banks). No files, no windows |
| `crates/cart-model` | What is known about the cartridge: levels, objects, world map, patches, driving the game |
| `crates/host` | Finding the cartridge, file locations, battery saves, PNG export |
| `game/` | The game → `game.exe` |
| `editor/` | The editor → `editor.exe` |
| `tools/research/` | Reverse-engineering command line → `research.exe` |
| `regression/` | Golden fingerprint test (behaviour must match the legacy port) |
| `legacy/gb_port/` | The original port, untouched, kept as reference until the refactor ends |
| `docs/ARCHITECTURE.md` | Target architecture and refactor phases |

## Build and run

    cargo build --release
    target\release\game.exe ["path\to\game.gb"] [--level N]
    target\release\editor.exe
    target\release\research.exe disasm 0 0361 20

The cartridge is found as before: a path on the command line, a `rom` folder next to the program
(or above it), then `%USERPROFILE%\Games\...`. Saves (`saves\`), edited copies (`edits\`) and
`settings.cfg` live next to the executables.

Checks:

    cargo test --release --workspace                  # includes the golden fingerprint
    target\release\game.exe --self-test out.json
    target\release\game.exe --research menushots DIR
    target\release\game.exe --research customcheck OUT.png

Controls and editor usage: see `legacy/gb_port/README.md` (unchanged).
