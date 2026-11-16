# editor/status-messages Specification

## Purpose
The editor's messages say what kind they are: information, something done, a warning or an
error, by colour, sign and weight, so a refusal or a failure is not read as a plain note.

## Requirements

### Requirement: Kinds of message
Every message the editor shows (status bar, catalogue panel, blank level chooser) SHALL be
information, done, warning or error. Information SHALL look as before; done SHALL be green
with "+", a warning orange with "!", an error red with "x", those three in bold. A message is
information unless the editor marks it otherwise.

#### Scenario: A long message
- **WHEN** a message is longer than the status bar or the side panel is wide
- **THEN** it goes on as many lines as it needs (the status bar grows upwards), nothing cut

#### Scenario: A refusal
- **WHEN** an enemy that does not fit is added to a level's set
- **THEN** the message that names the enemy it clashes with is orange, bold and starts with "!"

#### Scenario: A failed save
- **WHEN** the copy cannot be written
- **THEN** the message is red, bold and starts with "x"

#### Scenario: Back to information
- **WHEN** a warning is shown and then the user does something that gives a plain message
- **THEN** that message looks as information
