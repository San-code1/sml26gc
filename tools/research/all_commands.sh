#!/bin/sh
# usage: research_all.sh EXE OUTDIR  — runs every research command, one output file each
# Compare two builds: run it with each exe into two folders, then diff the folders
# (every file must be byte-identical; the pictures too).
EXE="$1"; OUT="$2"; mkdir -p "$OUT"; rm -f "$OUT"/*
SC="S@400-405;S@600-605;S@800-805;R@960-1060;BR@1080-1200;A@1260-1280;BR@1360-1500;A@1430-1500"
r() { name="$1"; shift; "$EXE" "$@" > "$OUT/$name.txt" 2>&1; }
r disasm0  disasm 0 0150 40
r disasm3  disasm 3 4037 30
r table    table 19ff 64 8
r levels   levels
r store    storecheck
r free     free
r objcheck objcheck
r world    worldcheck
r objects  objects 5
r find     find "$SC"
r watch    watch "$SC" ffc2,ffc3 960 1000
r trace    trace "$SC" ff9b,a269,a200-a201 900 960
r reads    reads "$SC" ff 4000 7fff 950 960
r modes    modes "$SC" t:300,m:850,l:1000,l:1050
r diff     diff "$SC" 800 1000 0c 04
r poke     poke "$SC" 1000 a299=00 1100 "$OUT/poke.png"
r leave    leave "$SC" 1000 map 1100 "$OUT/leave.png"
r cmpmap   cmpmap "$SC" 1100 "$SC" 1000 1100
r wshot    worldshot 10000 4096 64 "$OUT/wshot.png"
r looks    looks 0 "$OUT/looks.png"
r usage    nonsense
