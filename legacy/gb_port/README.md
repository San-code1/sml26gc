

# Game Boy cartridge, native on Windows (Rust)

A native Windows program written in Rust that runs the person's own Game Boy
cartridge file. Instead of redrawing the game, it implements the Game Boy
itself (processor, memory banks, screen, sound, timer, buttons) so the
cartridge's own program runs unchanged: every rule, speed, level and sound is
the original's.

The cartridge file is read at start-up from the person's own copy (it is not
copied into this folder). It is looked for, in order: a path given on the
command line, a `rom` folder next to the program or in a folder above it, then
`%USERPROFILE%\Games\Super Mario Land 2 - 6 Golden Coins (USA, Europe)`, then
any `.gb` file under `%USERPROFILE%\Games`.

## Controls

| Game Boy | Keyboard (default, changeable in the menu) |
|---|---|
| D-pad | Arrow keys |
| A (jump / swim) | X |
| B (run / fire / spin) | Z |
| Start | Enter |
| Select | Backspace |

Pause menu: Enter during a level (or Esc anywhere) freezes the game and opens a menu
drawn in the Game Boy's own four shades: Continue, Controls (press a key to rebind
each button; a key already in use swaps places; Restore defaults), Display (colours,
full screen, sharp or fitted scaling), Sound (volume 0-10), the game's original pause
(sends Start), Back to map (in a level only: done the way the game's own pause exits a
cleared level, routine 0:2774, so no life is lost), File select A/B/C (in a level or on
the map: restarts the cartridge like a power cycle, keeping the battery saves, and
presses Start on the title for you), Language (English by default, Spanish or Japanese in katakana), and Quit. On the title and map screens Enter stays the game's Start.
Settings are kept in `settings.cfg` next to the program. F11 toggles full screen,
hold Ctrl to fast-forward.

Battery saves (the game's own three save files) are written to `saves\` next to
the program every 5 seconds and when the window closes.

## Level editor (`editor.exe`)

A separate Rust program next to the game. It opens your cartridge, lets the game itself load
each of the 32 levels to get their real blocks and graphics, and shows them for editing:

- Level buttons 00-31 (a `*` marks unsaved changes); the whole level as a clickable overview (M).
- Block palette: the 128 blocks of that level's zone (ground, pipes, coins, ? blocks, bricks,
  spikes, the goal...). Left click paints, right click picks the block under the mouse.
- Tools: Pencil (B), Box fill (R), Grid (G), Undo/Redo (Ctrl+Z / Ctrl+Y), Original (restore the
  level as shipped). Pan with the arrows/WASD, the middle mouse button or Space+drag; zoom with
  the wheel.
- Save (Ctrl+S) writes `edits\<game> (edit).gb` next to the program; the original file is never
  changed. The copy is 1 MB so each zone has about 15 KB for its levels (the bar at the bottom
  shows how full the zone is). Play (F5) saves and opens the game straight in that level.
- Levels that share a map change together (the status bar says which).
- Objects mode (Tab): the enemies and other objects the game places in the level, shown as
  boxes with their type number (00-7F). Click empty space to place the chosen type, drag to
  move (8-pixel steps), right click or Del to delete, pick a type in the palette to change the
  selected object (dark types are the ones the level already uses). The bar shows the object
  space (about 8 KB in the edited copy, against 2.9 KB in the original cartridge).

- World map editor (button under the level list): the whole 512x512 world map with the
  game's own map tiles. "Picture" paints tiles (pencil, box, right click picks, undo, Original);
  "Level points" chooses which level (00-31, or none) each map point opens, so an edited or
  rebuilt level can be put on any point. Saved into the same edited copy.

- Object pictures: in Objects mode the editor lets the game show each object type on its own
  beside Mario in that level and cuts out the sprites it draws, so objects and the type palette
  show the real enemy/item pictures (filled in a few per second, the level's own types first).
  Types that draw nothing there (invisible triggers and the like) stay numbered boxes.

- Your own world map ("Your map" tab of the world map editor): put points anywhere on the
  world picture, join them with paths, choose the level each point opens (or none, for a
  junction) and where Mario starts. Click empty map = new point joined to the selected one;
  drag = move; right click = delete; "Join to..." then another point = add/remove a path.
  "Try it in the game" saves and opens the game. Saved as `edits\<game> (edit).points.txt`.

In the game, when that file exists for the cartridge being played, the port draws this map
instead of the cartridge's own when the world map comes up: the d-pad walks Mario along the
paths, A or Start enters the point's level, and after a level you come back to the same
point. Cleared levels are marked (filled square, `*` next to the level number) and kept in
`saves\<game>.progress.txt`; a level counts as cleared when the map comes back without a
lost life or "Back to map". The pause menu has "MAP: YOURS / MAP: ORIGINAL" to switch.
All paths are open from the start (the cartridge's unlocking of paths isn't used with your map).

## Build and run

    cargo build --release
    target\release\gb_port.exe ["path\to\game.gb"]

Options for checks: `--screenshot out.png` (draws 120 frames, saves the window,
exits; the window opens behind the others) and `--self-test out.json` (plays the
start of the first level with scripted buttons and reports move / jump / status bar).
