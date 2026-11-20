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

## OpenSpec

The specs (`openspec/specs/`) and the OpenSpec settings (`openspec/config.yaml`) are in the
repository. Changes being planned or applied (`openspec/changes/`) stay on the developer's
computer (`.gitignore`); archiving a change brings its result into the specs, which are
committed. A change is implemented on its feature branch.

## Tests

Tests are the last task of a feature, with or without OpenSpec: do not run them after every
edit. Once the user validates the work (the feature is done, or the OpenSpec change is ready to
archive), run `cargo test --release --workspace` (it includes the golden fingerprint in
`regression/`), fix what fails, then commit and close the feature.

## Language

Code, identifiers, comments, commit messages, READMEs and documentation are in English. Only
the user-facing texts of the programs are translated, only in the catalogues `*/i18n/*.txt`
read by `crates/i18n` (the code holds the English).

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

## Licences

The project is `GPL-3.0-or-later` (copyright The SML2 6GC authors): new files fall under it through
`REUSE.toml`. Anything brought in from elsewhere (code, fonts, art) must have a licence
compatible with the GPL, get its own entry in `REUSE.toml` and its licence text in `LICENSES/`.
Nothing of the game or of Nintendo goes in (`LEGAL.md`).

## Releases

A package for another computer is only made with `cargo xtask package <target>`
(`docs/DEVELOPMENT.md`, "Building for Windows"), never by copying from `target/release`, which holds the
developer's own cartridge choice, projects and saves.
