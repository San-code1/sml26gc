# Icons

Each program's icon, named like its executable: `sml26gc.png` (the game) and
`sml26gc_editor.png` (the editor). Square PNG, 256x256 or larger, transparent background; it
should still read at 16x16 (the title bar).

At build time `crates/app-resources` makes from each the window icon (16, 32, 64 pixels) and,
on Windows, the executable's `.ico` (16 to 256 pixels). A missing file only gives a build
warning and the system's icon.
