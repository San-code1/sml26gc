# Regression fingerprint

`golden.txt` is the behaviour of the legacy port (`legacy/gb_port`) recorded as `key = value`
lines: hashes of screens, sound, memory and ROM data, plus small values (offsets, sizes, flags).
It contains nothing from the cartridge itself, only hashes and numbers.

The refactored crates must reproduce it **line for line**.

| Prefix | What it covers |
|---|---|
| `rom.*`  | Level headers, map decode/encode, objects, world map, bank layout |
| `exp.*`  | ROM expansion to 1 MB, re-packing levels/objects, an edited level that still boots |
| `cmap.*` | The editor's own world-map text format |
| `emu.*`  | 3600 frames from power-on with scripted buttons: screen and sound every 60 frames |
| `uc.*`   | Boot to map, enter each of the 32 levels, run, back to map |

Run (needs your own ROM; set `SML2_ROM` or keep it where the game finds it):

    cd legacy/gb_port
    cargo test --release --test golden                    # compare
    $env:GOLDEN_UPDATE=1; cargo test --release --test golden   # rewrite (only on purpose)
