# release/platforms Specification

## Purpose
Which systems the editor and the game support, what running on one of them guarantees, and how the
code keeps what depends on the system apart so another platform can be added.

## Requirements

### Requirement: Supported platforms
The supported platforms SHALL be Windows 11 on x86-64 (`windows-x64`). Linux is planned and SHALL be added as a new platform target and new adapters, without moving the domain and application code.

#### Scenario: The targets the package command knows
- **WHEN** the package command lists its targets
- **THEN** it lists `windows-x64`

### Requirement: Runs without installing anything
On a supported platform, the unpacked executables SHALL start on a freshly installed system with no runtime, redistributable or library installed beforehand: the C runtime is part of each executable, and the fonts and translation catalogues are inside them. The only file the person adds is their cartridge.

#### Scenario: A fresh Windows 11
- **WHEN** the Windows package is unpacked on a Windows 11 computer without the Visual C++ Redistributable and `sml26gc.exe` is opened
- **THEN** the game starts and asks for its base cartridge, with no error about a missing DLL

### Requirement: Portable data next to the programs
The programs SHALL keep their settings, projects, saves, caches and exports in the folder they are run from, as they do now, so the unpacked folder is self-contained and can be moved or copied whole. When that folder cannot be written, the program SHALL say so in its own language instead of failing silently.

#### Scenario: Moving the folder
- **WHEN** the person moves the unpacked folder, with their projects and saves in it, to another drive
- **THEN** the editor and the game find their projects, saves and settings there

#### Scenario: A read-only folder
- **WHEN** the folder is unpacked where it cannot be written (for example under Program Files) and a setting is changed
- **THEN** the program shows a message that its folder cannot be written

### Requirement: No console window
On Windows, opening the editor or the game SHALL show only its own window, with no console window behind it; and the editor starting a test game SHALL not open one either. Their command-line modes (the game's `--research` checks and other modes that print) SHALL still print to the terminal they were started from.

#### Scenario: Double-clicking the game
- **WHEN** `sml26gc.exe` is opened from Explorer
- **THEN** only the game's window appears

#### Scenario: A check from a terminal
- **WHEN** `sml26gc.exe --research menushots DIR` is run from a terminal
- **THEN** its messages appear in that terminal

### Requirement: The system stays behind ports
Code that depends on the operating system (window menus, file dialogs, the system language, the data folder, starting the other program, the executables' names and resources) SHALL be in adapters behind ports, selected per platform at build time, each with a Windows implementation and a fallback for other systems that builds and keeps the program usable. The domain and application code SHALL build for any target.

#### Scenario: Adding a platform
- **WHEN** Linux support is added
- **THEN** the work is new adapters, a new package target and their tests, and no domain or application module changes
