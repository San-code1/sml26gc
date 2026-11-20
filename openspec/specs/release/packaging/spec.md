# release/packaging Specification

## Purpose
How the editor and the game are made into a folder and a zip that another person can unpack and run
on a supported platform, with only what the programs need and the notices they must carry.

## Requirements

### Requirement: One command builds a platform's package
A single command run from the repository, naming a platform target (`windows-x64` for now), SHALL build the editor and the game for that target in release and assemble the package folder `dist/<target>/sml26gc/`, then write `dist/sml26gc-<version>-<target>.zip` holding that folder. It SHALL start from an empty package folder each time, so nothing from an earlier package remains. An unknown target SHALL stop with a message listing the known ones, and nothing SHALL be written.

#### Scenario: Building the Windows package
- **WHEN** the developer runs the package command for `windows-x64`
- **THEN** `dist/windows-x64/sml26gc/` holds the editor and the game built in release, and `dist/sml26gc-<version>-windows-x64.zip` holds that folder

#### Scenario: An unknown target
- **WHEN** the package command is run for `amiga`
- **THEN** it says which targets it knows and writes nothing

### Requirement: The package holds only what runs
The package folder SHALL hold exactly: the game and editor executables, a read-me, the cartridge notice, the licence text, the legal notice, the changelog, the user manuals of the game and the editor, and the licences folder. It SHALL NOT hold source files, build output (debug symbols, intermediate files, other binaries such as the research tool), cartridges or anything made from one, projects, saves, settings files, caches or exports.

#### Scenario: The developer's own files stay out
- **WHEN** the package is built on a computer whose `target/release` holds projects, saves, `settings.cfg`, `editor.cfg`, `base_cartridge.txt` and `.pdb` files
- **THEN** none of them is in the package folder or the zip

#### Scenario: Listing the package
- **WHEN** the Windows package folder is listed
- **THEN** it holds `sml26gc.exe`, `sml26gc_editor.exe`, `README.txt`, `NO-CARTRIDGE.txt`, `LICENSE.txt`, `LEGAL.md`, `CHANGELOG.md`, `manuals/` (`GAME.md`, `EDITOR.md`) and `licenses/`, and nothing else

### Requirement: No cartridge is included
The package SHALL NOT contain any cartridge or data taken from one, and SHALL say so: `NO-CARTRIDGE.txt` (and the read-me) SHALL state that the programs include no game, that the person uses a cartridge file from their own backup of a cartridge they own, that the authors take no responsibility for where it was obtained, and that the product is not affiliated with or endorsed by Nintendo. The programs SHALL keep asking for the cartridge on first run, as they do now.

#### Scenario: First run from the package
- **WHEN** the unpacked game is started on a computer that never ran it
- **THEN** it asks for the base cartridge file, and the package's notice explains that the person provides it

### Requirement: The executables carry their icons and version
On Windows, each executable SHALL show its own icon (in Explorer, the taskbar and its window) made from the PNG the project keeps for it: `assets/icons/sml26gc.png` for the game and `assets/icons/sml26gc_editor.png` for the editor. Each SHALL carry version details: the product name, the program's name, the version, and a line saying it is not affiliated with Nintendo. While an icon file is missing, the build SHALL still succeed, without that icon, and say which file is missing.

#### Scenario: The icons are given
- **WHEN** `assets/icons/sml26gc.png` and `assets/icons/sml26gc_editor.png` exist and the package is built
- **THEN** `sml26gc.exe` and `sml26gc_editor.exe` each show their own icon in Explorer, and their Properties show the product name and the version

#### Scenario: An icon is missing
- **WHEN** `assets/icons/sml26gc_editor.png` does not exist and the package is built
- **THEN** the build succeeds, the editor has the default icon, and the build output names the missing file

### Requirement: The package carries its licences
The `licenses/` folder SHALL hold the licence of every font the programs include (SIL Open Font License, one file per font family) and a list of the Rust crates compiled into the executables with each crate's licence text. The package command SHALL make this list from the build's own dependency graph, so it never names a crate the executables do not include.

#### Scenario: A font's licence
- **WHEN** the package is built
- **THEN** `licenses/` holds the licences of BIZ UDGothic, Noto Sans CJK and Fusion Pixel Font, and the third-party crates list names `macroquad` with its licence

### Requirement: The product's names
The product SHALL be named "SML2 6GC Emulator and Level Editor" (saying it is an emulator with a level editor for Super Mario Land 2: 6 Golden Coins), with the short id `sml26gc`. The game's executable SHALL be `sml26gc` (`sml26gc.exe` on Windows) and the editor's `sml26gc_editor` (`sml26gc_editor.exe`); the editor SHALL start the game, and the game the editor, by these names from their own folder.

#### Scenario: The editor starts a test game
- **WHEN** Play is chosen in the editor of an unpacked package
- **THEN** `sml26gc.exe` from the same folder starts on that level

### Requirement: Product name and version in one place
The product name, the short id, the version and the list of platform targets SHALL each be defined once (the version is the workspace's); the package's folder and zip names and the executables' version details SHALL take them from there. The package's texts (read-me, cartridge notice, third-party list) SHALL NOT show the version.

#### Scenario: A new version
- **WHEN** the workspace version is changed to `0.2.0` and the package is built
- **THEN** the zip is named with `0.2.0`, both executables' Properties show `0.2.0`, and no text of the package names a version

### Requirement: The package says what a computer needs
The package's read-me SHALL say what a computer needs to run the programs and that nothing else needs installing, and SHALL say, for each way a program can fail to start or work, its cause and what to do.

#### Scenario: A computer without a graphics driver
- **WHEN** the game closes at once on a fresh Windows 11 without its graphics driver
- **THEN** the package's read-me names the cause and how to get the driver or the OpenGL Compatibility Pack

#### Scenario: What the read-me covers
- **WHEN** the read-me is read
- **THEN** it gives the Windows version, the graphics driver (OpenGL 3.1, at least 2.1), the optional sound device and controller, how to see a program's messages from a terminal, and what to do for SmartScreen, a missing or old graphics driver (the maker's driver, 3D acceleration in a virtual machine, the OpenGL Compatibility Pack, Remote Desktop), a missing Visual C++ runtime (the redistributable to download), a folder that cannot be written, a refused cartridge, no sound and a controller that does nothing

### Requirement: Documentation for players and developers
The repository SHALL document, each in one place: what the game adds to the original and how to use it (a user manual), what the editor lets a person do and how (a user manual), how to prepare a computer to develop (the language, tools and how to install them), how to build, check and package for Windows, and how and why the emulator and the editor were made. The README SHALL describe both programs briefly and link to each document.

#### Scenario: A new developer
- **WHEN** someone with a fresh Windows 11 reads the development guide
- **THEN** it names every tool to install (Rust with the MSVC toolchain, the Visual Studio Build Tools with the C++ workload and a Windows SDK, Git, and the optional OpenSpec CLI), where to get each, and the commands to build, test and package

### Requirement: Licence
The project's own work SHALL be licensed under GPL-3.0-or-later, copyright The SML2 6GC authors, with each file's licence stated in `REUSE.toml` and the texts in `LICENSES/` (the fonts keep OFL-1.1). Each package SHALL carry the licence text and the address of the repository that holds the source code, and the executables' version details SHALL name the copyright holder and the licence.

#### Scenario: Someone receives the zip
- **WHEN** a person unpacks a package
- **THEN** it holds `LICENSE.txt` (the GPL) and `LEGAL.md`, and its read-me gives the copyright, the licence and the address of the source code

#### Scenario: Which licence a file has
- **WHEN** a font file or a source file is looked up in `REUSE.toml`
- **THEN** the font is OFL-1.1 with its authors, and the source file GPL-3.0-or-later, copyright The SML2 6GC authors

### Requirement: Legal notice
A legal notice (`LEGAL.md`), also in each package, SHALL state what is and is not included, the legal grounds of the emulator and the editor, that the project is not commercial, who is responsible for what, that there is no warranty, and how a rights holder reaches the author.

#### Scenario: What the legal notice covers
- **WHEN** `LEGAL.md` is read
- **THEN** it says no game or Nintendo material is included, gives the grounds (study and interoperability of programs, the user's own backup, changes for one's own use, names used only to say what the programs work with), says nobody makes money from the project, that each person answers for their use and their cartridge file's origin, and gives the author's contact
