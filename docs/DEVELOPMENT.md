# Development

How to set up a computer to work on the project, build and check it, make the Windows package,
and how the emulator and the editor were made. The rules for changing the code are in
[`AGENTS.md`](../AGENTS.md); the code's layout in [`ARCHITECTURE.md`](ARCHITECTURE.md).

## What you need

Development happens on Windows 11 (x64); the Windows package is built there.

| Tool | Why | How to get it |
|---|---|---|
| Rust (stable, MSVC toolchain `stable-x86_64-pc-windows-msvc`) | The language of every program (edition 2021; built with 1.100) | [rustup](https://rustup.rs): run `rustup-init.exe`, keep the default (MSVC) toolchain; `rustup update` later |
| Visual Studio 2022 Build Tools, workload **Desktop development with C++** (MSVC v143 and a Windows 11 SDK) | The linker (`link.exe`) Rust uses on Windows, and the resource compiler (`rc.exe`) that puts the icons and version details in the executables | [Visual Studio downloads](https://visualstudio.microsoft.com/downloads/), "Build Tools for Visual Studio 2022" (rustup offers it when it is missing) |
| Git | The repository and its workflow (`AGENTS.md`) | [git-scm.com](https://git-scm.com) (Git Bash runs `tools/research/all_commands.sh`) |
| Your own Super Mario Land 2 - 6 Golden Coins (USA, Europe) cartridge file | Running the programs, the golden fingerprint test and the research tool | Your backup of your cartridge. Never put it, or anything made from it, in the repository |
| Optional: Node.js and the OpenSpec CLI (`npm install -g @fission-ai/openspec`) | Planning changes as specs (`openspec/`) | [nodejs.org](https://nodejs.org) |
| Optional: the Linux target (`rustup target add x86_64-unknown-linux-gnu`) | `cargo xtask check-portable` | rustup |

Every other dependency (macroquad, cpal, gilrs, png...) is a Rust crate Cargo downloads by itself.
No Python, CMake or other build system is needed.

## Build and run

    cargo build --release
    target\release\sml26gc.exe ["path\to\game.gb"] [--project NAME | --test] [--level N] [--cheats]
    target\release\sml26gc_editor.exe

The emulator is too slow unoptimised, so even `cargo build` (debug) is optimised; use `--release`
for playing. `.cargo/config.toml` links the C runtime into every Windows build.

Both programs keep their files next to themselves, in `target\release\`: the base cartridge
choice (`base_cartridge.txt`), `settings.cfg`, `editor.cfg`, the projects (`editor\projects\`),
the saves (`saves\`), `exports\` and `cache\`. `cargo clean` deletes them with the build: copy
them out first.

## Tools

| Command | What it does |
|---|---|
| `cargo test --release --workspace` | Every test, with the golden fingerprint (`regression/`, see its README): needs the cartridge (`SML2_ROM`, or where the game finds it) |
| `$env:GOLDEN_UPDATE=1; cargo test --release -p regression --test golden` | Rewrites the fingerprint, only for an intended behaviour change |
| `target\release\research.exe COMMAND ...` | The reverse-engineering tool (`tools/research`): disassembly, memory watch and trace, ROM views, checks that play edited data. The cartridge comes from the command line, `RESEARCH_ROM`, the base cartridge, a `rom` folder or `%USERPROFILE%\Games` |
| `sh tools/research/all_commands.sh EXE DIR` | Runs a fixed set of research commands into `DIR`, to compare two builds file by file |
| `sml26gc.exe --research menushots DIR` | The pause menu's screens in every language, as pictures |
| `sml26gc.exe --self-test out.json` | Checks the game's rules read from the cartridge (`MECHANICS.md`) against the running game |
| `sml26gc.exe --dump DIR --script ... --frames N --every N` | Screens of a scripted run |
| `sml26gc.exe --screenshot FILE [--start-screen]`, `sml26gc_editor.exe --screenshot FILE [--project NAME --level N]` | One picture of the window, with default settings (unattended checks) |
| `cargo xtask targets`, `cargo xtask package TARGET`, `cargo xtask check-portable` | The release tool, below |

Never run `cargo fmt`: the code is formatted by hand (`AGENTS.md`).

## Building for Windows

The package another person unpacks and runs:

    cargo xtask package windows-x64

1. Builds the game and the editor with the `dist` profile (link-time optimisation, no debug
   symbols) for `x86_64-pc-windows-msvc`, in `target\x86_64-pc-windows-msvc\dist\` (Cargo's
   build folder, named after the profile; never run the programs from it).
2. Checks that neither executable needs the Visual C++ runtime DLLs (`VCRUNTIME140.dll`,
   `api-ms-win-crt-*`): `.cargo/config.toml` links the C runtime in (`+crt-static`).
3. Writes `dist\windows-x64\sml26gc\`, emptied first: `sml26gc.exe`, `sml26gc_editor.exe`,
   `README.txt` and `NO-CARTRIDGE.txt` (from `packaging/`, with the product and program names
   filled in), `LICENSE.txt` (the GPL), `LEGAL.md`, `CHANGELOG.md`, `manuals\` (`docs/GAME.md`,
   `docs/EDITOR.md`) and `licenses\` (the fonts' and those of every Rust crate compiled in, from
   `cargo metadata`).
4. Zips that folder to `dist\sml26gc-<version>-windows-x64.zip`.

What goes into the executables:

- The icons: `assets/icons/sml26gc.png` and `sml26gc_editor.png` (see its README), made into
  the `.ico` and the window icon by `crates/app-resources` at build time. A missing PNG is a
  build warning; a missing `rc.exe` (no Windows SDK) leaves the executables without icon and
  version details, with a warning.
- The version details: the product name (`PRODUCT_NAME` in `.cargo/config.toml`), the version
  (the workspace's `version` in `Cargo.toml`) and the not-affiliated line.
- The fonts and every translation catalogue (nothing is read beside the executable but the
  person's own files).
- The window subsystem: no console window; `host::console::attach()` gives the command-line
  modes the terminal they were started from.

A package is published only as part of a release (below, "Workflow and releases"). What a
computer needs to run it, and what to do when it does not start, is in
[`packaging/README.txt`](../packaging/README.txt).

Build errors and their cause:

| Error | Cause and fix |
|---|---|
| `linker 'link.exe' not found` | The Build Tools are missing: install the C++ workload |
| `warning: ... no icon or version details` | No Windows SDK (`rc.exe`): add a Windows 11 SDK in the Visual Studio Installer |
| `... needs VCRUNTIME140.dll` from the xtask | `.cargo/config.toml` was not applied (built from another folder, or `RUSTFLAGS` set in the environment, which overrides it) |
| `error: failed to remove file ... .exe` | The program is open: close it and build again |

Other platforms: the code is ready for them (see `ARCHITECTURE.md`, "Platforms and packaging");
each is built on its own system.

## Workflow and releases

The rules are in [`AGENTS.md`](../AGENTS.md), "Git workflow"; this is why they are so.

- **Two long-lived branches.** `main` only ever holds released versions, so whatever is on it
  can be built and given to someone; `develop` gathers finished features until the next release.
- **One branch per change, from `develop`, even for an urgent fix.** Every change goes through
  the same steps (its spec, its tests, the changelog), so a hurried fix cannot skip them.
  A release then carries everything already on `develop`, so its number is chosen by all it
  holds, not only by the fix.
- **Archive, changelog and tests on the branch.** The branch arrives at `develop` complete: the
  specs say what the code now does, the changelog says what users will notice, the tests pass.
  Writing the changelog when each feature ends is easier and truer than rebuilding it from
  memory at release time, and its "Unreleased" section tells which number the release needs.
- **`--no-ff`, then delete the branch.** The merge commit keeps the feature visible as one unit
  in the history, so the branch itself is no longer needed; old branches only hide the ones that
  matter.
- **The version changes once, at release time.** It is what the executables, the zip and the tag
  show, so it changes in the commit that prepares the release and nowhere else. It is
  `MAJOR.MINOR.PATCH`: PATCH when a release only fixes, MINOR when it adds features, MAJOR when
  projects or saves of an earlier version no longer work (while it is `0.x`, the programs are
  still maturing).
- **The tag is the version and points at the merge on `main`.** A published zip can always be
  traced to the exact code it was built from. A release may have a name too: it is its title on
  GitHub, never its tag.

Publishing release X.Y.Z, once the user has asked for it and agreed the number:

1. On `develop`: set `version = "X.Y.Z"` in the workspace `Cargo.toml`, build (it updates
   `Cargo.lock`), turn `## [Unreleased]` in `CHANGELOG.md` into `## [X.Y.Z] - YYYY-MM-DD` with a
   new empty "Unreleased" above it, run the tests, commit "Release X.Y.Z", push.
2. `git checkout main`, `git merge --no-ff develop`, `git tag -a vX.Y.Z -m "Version X.Y.Z"`,
   `git push origin main vX.Y.Z`.
3. `cargo xtask package windows-x64` (from `main`): `dist\sml26gc-X.Y.Z-windows-x64.zip`.
4. On GitHub, Releases, "Draft a new release": choose the tag `vX.Y.Z`, paste the changelog's
   section as the text, attach the zip, publish (or `gh release create vX.Y.Z <zip> --notes-file
   <section>` with the GitHub CLI).

## How the emulator was made, and why

The game is not rewritten: the original cartridge's program runs, unchanged, on a Game Boy
written in Rust (`crates/gb-core`): the SM83 processor, the memory map and the cartridge's bank
controller, the picture processor drawing tiles and sprites line by line, the four sound
channels, the timer and the buttons. Every rule, speed, level and sound is then exactly the
game's own, with nothing to copy into the project: the person's cartridge file is the game.

What the port adds lives around the emulated machine, never inside the game's program unless
needed: the window, scaling and palettes, sound output, keyboard and controller with any
bindings, the pause menu in eleven languages, settings, the start screen, save states and rewind
(snapshots of the whole machine), fast forward, and the cheats, which change the running game's
memory or apply small patches to the copy in memory (`cart-model`'s `cheats`).

To know what to change, the cartridge's program was read with the research tool (an SM83
disassembler, write-watches naming the instruction that wrote a value, traces), and what was
learned is written down: `MECHANICS.md` (the rules) and `cart-model` (where levels, objects,
maps, routes and doors are, with ROM addresses). The golden fingerprint (`regression/`) holds the
emulator and that knowledge to their recorded behaviour, so any change that alters them shows.

## How the editor was made, and why

The editor changes the game's own data, in the game's own formats, and the game's own program
plays it: a level is the compressed block map, the object list and the header the cartridge
already uses; the world map is its picture, its points, the walk scripts between them and the
doors. Edits are never drawn by the port: they are written into a copy of the cartridge, which
any faithful Game Boy emulator could run. This keeps the original behaviour (physics, enemies,
music) for every edited level, and lets the editor show each level exactly as the game draws it.

- **Reading**: `cart-model` decodes the levels, objects and maps from the cartridge; the pictures
  of blocks and objects are captured from the running game (the editor runs the emulator in the
  background), so they are the game's own graphics in their real colours.
- **Room**: the original 512 KB cartridge is full, so the edited copy is 1 MB: twin banks give
  levels and objects more room, and new levels (32 and up) and new zones live in the added banks.
- **New behaviour**: what the game cannot do by itself (checkpoints with several bells, secret
  exits that lead anywhere, more map points, new zones, a different starting point) is added by
  small patches ("hooks") in the copy's code, each documented in `cart-model` with its address.
- **Projects**: what the person edits is kept as a project file (text, over the base
  cartridge), and the copy is made from it whenever it is missing or older; the base cartridge is
  never written. The game lists the projects on its start screen; the editor starts test games
  of the level being edited.
- **Checks**: the editor warns about what would not work in the game (too many objects, objects
  where the game never wakes them, missing goals), and the research tool's checks play edited data
  in the emulator to prove routes, doors and new zones work.

The code follows a hexagonal architecture: the rules (`domain`) and use cases (`application`)
know no files, windows or sound; adapters do (`ARCHITECTURE.md`).
