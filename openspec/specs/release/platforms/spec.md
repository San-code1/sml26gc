# release/platforms Specification

## Purpose
Which systems the editor and the game support, what running on one of them guarantees, and how the
code keeps what depends on the system apart so another platform can be added.

## Requirements

### Requirement: Supported platforms
The supported platforms SHALL be Windows 11 on x86-64 (`windows-x64`) and Linux on x86-64 (`linux-x64`), for desktop distributions with glibc 2.31 or newer (Debian 11, Ubuntu 20.04 and later) and an X11 or Wayland (through Xwayland) desktop. Each platform's package SHALL be built on that platform; the Linux one inside a Debian 11 container (see packaging).

#### Scenario: The targets the package command knows
- **WHEN** the package command lists its targets
- **THEN** it lists `windows-x64` and `linux-x64`

#### Scenario: An older distribution
- **WHEN** the Linux package is unpacked on Ubuntu 20.04 or Debian 11 and `./sml26gc` is run
- **THEN** the game starts, with no error about the glibc version

#### Scenario: A Wayland desktop
- **WHEN** the game is started on Ubuntu 24.04 with its default Wayland session
- **THEN** its window opens and plays as on an X11 session

### Requirement: Runs without installing anything
On a supported platform, the unpacked executables SHALL start on a freshly installed system with no runtime, redistributable or library installed beforehand: the C runtime is part of each executable on Windows, and on Linux they need only libraries a desktop distribution installs by default (glibc, ALSA, udev, OpenGL and X11). The fonts and translation catalogues are inside them. The only file the person adds is their cartridge.

#### Scenario: A fresh Windows 11
- **WHEN** the Windows package is unpacked on a Windows 11 computer without the Visual C++ Redistributable and `sml26gc.exe` is opened
- **THEN** the game starts and asks for its base cartridge, with no error about a missing DLL

#### Scenario: A fresh Ubuntu desktop
- **WHEN** the Linux `.tar.gz` is unpacked on a freshly installed Ubuntu 24.04 desktop and `./sml26gc` is run
- **THEN** the game starts and asks for its base cartridge, with no error about a missing library

### Requirement: Portable data next to the programs
The portable packages' programs SHALL keep their settings, projects, saves, caches and exports in the folder they are run from, so the unpacked folder is self-contained and can be moved or copied whole; only the installed Linux programs keep them in the user's folder (see `release/linux-install`). When the data folder cannot be written, the program SHALL say so in its own language instead of failing silently.

#### Scenario: Moving the folder
- **WHEN** the person moves the unpacked folder, with their projects and saves in it, to another drive
- **THEN** the editor and the game find their projects, saves and settings there

#### Scenario: A read-only folder
- **WHEN** the folder is unpacked where it cannot be written (for example under Program Files, or a folder of another user on Linux) and a setting is changed
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
Code that depends on the operating system (window menus, file dialogs, the system language, the data folder, starting the other program, the executables' names and resources) SHALL be in adapters behind ports, selected per platform at build time, each with a Windows and a Linux implementation and a fallback for other systems that builds and keeps the program usable. The domain and application code SHALL build for any target.

#### Scenario: Adding a platform
- **WHEN** Linux support is added
- **THEN** the work is new adapters, a new package target and their tests, and no domain or application module changes

### Requirement: Built with the current Rust
The workspace SHALL use the current Rust edition (2024) with its dependency resolver (3), and declare as `rust-version` the stable toolchain it is built and tested with (1.100). Every crate SHALL take the edition from the workspace. The whole workspace SHALL build without warnings, and the tests, the regression fingerprint included, SHALL pass unchanged: the edition changes the code's language rules, never what the programs do.

#### Scenario: Building with the declared toolchain
- **WHEN** the workspace is built and tested with Rust 1.100
- **THEN** it builds without warnings and every test passes, with the regression fingerprint matching `regression/golden.txt`

#### Scenario: An older toolchain
- **WHEN** someone builds with a Rust older than 1.100
- **THEN** Cargo says the project needs Rust 1.100

### Requirement: The system's file dialogs on Linux
On Linux, choosing the base cartridge (in the game and the editor) and opening or saving level, zone, world map and project files in the editor SHALL use the desktop's own file dialog, titled in the program's language, with the same file kinds as on Windows. When the desktop offers no dialog, the program SHALL say so in its own language and keep working; the game SHALL still accept the cartridge as its command-line argument.

#### Scenario: Choosing the cartridge on Linux
- **WHEN** the game asks for its base cartridge on Ubuntu 24.04 and the person chooses Browse
- **THEN** the desktop's file dialog opens, showing Game Boy cartridge files, and the chosen file becomes the base cartridge

#### Scenario: Exporting a level on Linux
- **WHEN** Ctrl+E is pressed in the level editor on Linux
- **THEN** the desktop's Save dialog for a level file opens, with the level's file name suggested

#### Scenario: No dialog available
- **WHEN** Browse is chosen on a Linux system with neither a desktop portal nor zenity
- **THEN** the program shows a message that no file dialog is available and that the cartridge can be given as an argument

### Requirement: The system language on Linux
On Linux, the system language SHALL be read from `LC_ALL`, then `LC_MESSAGES`, then `LANG` (the first one set and not empty), and SHALL be English when none names a language the programs have.

#### Scenario: A Spanish desktop
- **WHEN** the game starts for the first time with `LANG=es_ES.UTF-8` and no `LC_ALL` or `LC_MESSAGES`
- **THEN** its texts are in Spanish

#### Scenario: LC_ALL wins
- **WHEN** `LC_ALL=fr_FR.UTF-8` and `LANG=es_ES.UTF-8`
- **THEN** the system language is French
