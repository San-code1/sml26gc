# AGENTS.md

Guidelines for anyone (people or AI agents) changing this repository.

## Project

A Game Boy emulator in Rust that plays the user's own cartridge, plus an editor that writes an
edited copy of it. The original game is always the base: the editor changes the game's own
levels, map, paths and doors, and the game's own code plays them. Cartridges, saves and edited
copies never go into the repository.

Layout and dependency rules: `docs/ARCHITECTURE.md`.

## Code style

- Rust, organised with SOLID principles and a hexagonal architecture (domain, application,
  ports, adapters). Domain code has no files, windows or audio; dependencies point inwards.
- SOLID with judgement: no `dyn` in the emulator's hot paths, closed sets stay `enum`s.
- Code is hand-formatted with long lines. **Never run `cargo fmt`.**
- Names should make the code self-explanatory. Write a comment only when the code cannot say
  it: ROM addresses, reverse-engineered formats, non-obvious constraints. Keep it short. No
  comments that repeat the code, section banners or history ("before", "now", "changed").
- Every change keeps `cargo test --release --workspace` passing, including the golden
  fingerprint in `regression/`.

## Language

Code, identifiers, comments, commit messages, READMEs and documentation are in English. Only
the user-facing texts of the programs are translated (`tr(en, es)`, `i18n.rs`).

## Documentation

Only what is needed to understand, build, run and change the project. It is not a development
diary: no history, plans or progress notes. Keep it short, current and without repetition;
say each thing in one place and link to it.

## Git workflow

- `main`: releases. `develop`: integration. Work happens on `feature/<name>` branched from
  `develop`.
- A finished feature is merged into `develop` with `git merge --no-ff` and pushed. `develop` is
  merged into `main` (`--no-ff`) only when the user asks.
- Commit messages: a short summary line, then what changed and why when it is not obvious.
- No AI attribution in commits or pull requests (no `Co-Authored-By` or "Generated with" lines).
- Never commit cartridges or anything derived from them (see `.gitignore`).
