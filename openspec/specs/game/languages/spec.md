# game/languages Specification

## Purpose
The game's own texts (its pause menu and its start screen) in eleven languages: which ones,
how one is chosen and kept, and that the pixel font draws every character they use.

## Requirements

### Requirement: Eleven languages in the game
The game's menu (every screen: main, Controls, Display, Sound, Language, confirmations, cheats,
summon, levels) and the start-up tags SHALL be in the language chosen: English, Spanish,
Japanese, Portuguese, French, German, Italian, Indonesian, Russian, Simplified Chinese or
Korean. A text a language lacks SHALL NOT exist: every language SHALL have every text.

#### Scenario: Korean
- **WHEN** Korean is chosen on the Language screen
- **THEN** the pause menu's entries, its title and its footer read in Korean

#### Scenario: Every text
- **WHEN** the game's texts are checked
- **THEN** each of the eleven languages has every menu text

### Requirement: Choosing the game's language
The Language screen SHALL list the eleven languages, each written in itself, scrolling when
they do not fit, and the one chosen SHALL apply at once and be kept in `settings.cfg`. A
`settings.cfg` from before keeps its language; an unknown language id SHALL fall back to
English.

#### Scenario: Chosen and kept
- **WHEN** German is chosen and the game is closed and started again
- **THEN** the menu is in German

#### Scenario: A list longer than the screen
- **WHEN** the cursor goes down the Language screen past the last language shown
- **THEN** the list scrolls to show it, as the Controls screen does

### Requirement: The start screen speaks the language
The game's start screen (projects to play, the base cartridge question, errors) SHALL use the
game's language: the eleven, where it used to speak only English or Spanish.

#### Scenario: Russian
- **WHEN** the game's language is Russian and the start screen shows that no cartridge is chosen
- **THEN** that message reads in Russian

### Requirement: Every character drawn by the pixel font
Every character of every language's menu texts SHALL have a glyph in the game's pixel font:
Latin capitals (the Latin languages written without accents, as Spanish is, but for its Ñ),
katakana, Cyrillic capitals, and the Chinese and Korean characters the texts use, the latter
8 pixels high; no character SHALL be drawn as a blank. Key and controller button names stay
as they are in English (Spanish keeps its own), as for Japanese.

#### Scenario: Chinese
- **WHEN** the menu is in Simplified Chinese
- **THEN** every character of its texts is drawn, none left blank

#### Scenario: A checked font
- **WHEN** the game's texts are checked
- **THEN** every character they use has a glyph in the pixel font

### Requirement: Choosing the language on the start screen
The game's start screen SHALL let the player choose the language in a drop-down list of the eleven (each written in itself), applied at once to the start screen and to the game started from it, and kept in `settings.cfg`. Until a language has been chosen (in the start screen or the menu), the game SHALL take the system's when it is one of the eleven, English otherwise.

#### Scenario: First start on a French Windows
- **WHEN** the game starts for the first time on a Windows set to French
- **THEN** the start screen and the menu are in French

#### Scenario: Chosen on the start screen
- **WHEN** the player chooses Korean on the start screen and plays
- **THEN** the game's menu is in Korean, and the next start opens in Korean

### Requirement: A test game speaks the editor's language
A test game started from the editor (Play, Play here, Play from a point, in levels and zones) SHALL use the editor's language for this game only; the game's own language setting SHALL stay as it was.

#### Scenario: The editor in German
- **WHEN** the editor is in German and the user presses Play
- **THEN** the game's menu is in German, and the game started on its own keeps its language

### Requirement: No Back rows
The menu's screens whose footer names the key that goes back (Display, Sound, Language, Debug / cheats) SHALL have no Back row; that key goes back as before.

#### Scenario: Sound
- **WHEN** the Sound screen is open
- **THEN** it shows the volume only, and Esc goes back to the main menu

### Requirement: The game's texts in editable catalogues
Every translated text of the game (its menu, its start screen, the key names that differ by language) SHALL be kept in plain-text catalogues (`game/i18n/*.txt`, the editor's format: an entry per English text with a line per language), read when the game starts, so a translation is changed by editing a text file; the code SHALL hold only the English.

#### Scenario: Correcting a translation
- **WHEN** a translator changes the French line of the entry "SOUND" in `game/i18n/menu.txt` and the game is built again
- **THEN** the French menu shows the new text, and no Rust file was edited
