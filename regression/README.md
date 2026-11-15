# Regression fingerprint

`golden.txt` records the emulator's and cartridge code's behaviour as `key = value` lines:
hashes of screens, sound, memory and ROM data, plus small values. It holds nothing from the
cartridge except hashes and numbers.

| Prefix | Covers |
|---|---|
| `rom.*`  | Level headers, map decode/encode, objects, world map, bank layout |
| `exp.*`  | 1 MB expansion, re-packing levels and objects, an edited level that still boots |
| `emu.*`  | 3600 frames from power-on with scripted buttons: screen and sound every 60 frames |
| `uc.*`   | Boot to map, enter each of the 32 levels, run, back to map |

Needs your own ROM (`SML2_ROM`, or where the game finds it); without it the test is skipped.

    cargo test --release -p regression --test golden                        # compare
    $env:GOLDEN_UPDATE=1; cargo test --release -p regression --test golden  # rewrite, on purpose only
