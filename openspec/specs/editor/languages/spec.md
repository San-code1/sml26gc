# editor/languages Specification

## Purpose
The editor's language: English, Spanish or Japanese, taken from the system until the person
chooses one, every text of the editor shown in it.

## Requirements

### Requirement: Three languages
Every text the editor shows (panels, buttons, tooltips, status bar, warnings, project screen,
choosers, menu bar, file dialogs) SHALL be in the language in use: English, Spanish or
Japanese. A text a language has no translation for SHALL show in English.

#### Scenario: Japanese
- **WHEN** the editor is in Japanese
- **THEN** the level editor's title, its buttons and the File menu read in Japanese

#### Scenario: A missing translation
- **WHEN** a text has no Japanese translation and the editor is in Japanese
- **THEN** that text shows in English and the rest in Japanese

### Requirement: The system's language first
Until a language is chosen, the editor SHALL use the system's: Spanish when the system's
display language is Spanish (any country), Japanese when it is Japanese, English otherwise.
The editor SHALL NOT follow the game's language setting.

#### Scenario: A Spanish Windows
- **WHEN** the editor starts for the first time on a Windows set to Spanish (Mexico)
- **THEN** it is in Spanish

#### Scenario: Another language
- **WHEN** the editor starts for the first time on a Windows set to French
- **THEN** it is in English

#### Scenario: The game in another language
- **WHEN** the game's menu is set to Japanese and the editor has no language chosen on an English Windows
- **THEN** the editor is in English

### Requirement: Choosing the language
The language chosen in the Language menu SHALL apply at once, to the menu bar too, without
losing anything being edited, and SHALL be kept in `editor.cfg` for the next start.
Automatic SHALL go back to the system's language.

#### Scenario: Changed while editing
- **WHEN** the language is changed with unsaved changes in a level
- **THEN** the texts change at once and the changes are still there, unsaved

#### Scenario: Kept
- **WHEN** Japanese is chosen and the editor is closed and opened again
- **THEN** it opens in Japanese

### Requirement: Japanese is readable
In Japanese every character the editor's texts use SHALL be drawn (no empty boxes or
question marks), at the sizes the editor uses, and the texts SHALL fit their buttons as
the English ones do (made smaller when they do not, as for any label).

#### Scenario: Every character
- **WHEN** the editor is in Japanese
- **THEN** every character of every Japanese text has its glyph in the editor's Japanese font
