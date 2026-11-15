# Credits and original files

Original files used (read at run time from the person's own installed copy, not copied into the project):

| File | Source path | Used for |
|---|---|---|
| Super Mario Land 2 - 6 Golden Coins (USA, Europe).gb (524,288 bytes, SHA-256 30d3316d06710cb94c529c477975cae3b051312b88aca9392888614b1c4b3894) | %USERPROFILE%\Games\Super Mario Land 2 - 6 Golden Coins (USA, Europe)\Super Mario Land 2 - 6 Golden Coins (USA, Europe).gb | The whole game: its program, graphics, levels, music and sound effects, executed by the Rust Game Boy in this project |

The game and its contents belong to their rights holders. Everything in `src/`
(the Game Boy processor, memory, screen, sound and timer, and the Windows front end)
was written for this project. Libraries: macroquad (window and drawing), cpal (sound
output), windows-sys (keeping the check window in the background).
