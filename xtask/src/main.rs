//! The release tool, run as `cargo xtask <command>` (the alias is in `.cargo/config.toml`):
//!
//!   targets               the platforms a package can be built for
//!   package TARGET        builds the game and the editor for TARGET and writes
//!                         `dist/TARGET/<id>/` and `dist/<id>-<version>-TARGET.zip`
//!   check-portable        checks the platform-independent crates for Linux
//!
//! A package holds the two executables, the read-me and the cartridge notice (`packaging/`),
//! the licence (`LICENSE`), the legal notice (`LEGAL.md`) and the changelog, the manuals (`docs/GAME.md`,
//! `docs/EDITOR.md`) and the other licences, and nothing else: it is built into its own folder under `target/`, apart
//! from the developer's `target/release` and the files the programs write there.
mod licences;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// A platform a package can be built for.
struct Target {
    name: &'static str,
    triple: &'static str,
    /// Libraries an executable must not import (they are not on every computer).
    forbidden_imports: &'static [&'static str],
}

const TARGETS: &[Target] = &[Target { name: "windows-x64", triple: "x86_64-pc-windows-msvc", forbidden_imports: &["vcruntime", "api-ms-win-crt-", "msvcp"] }];

/// The programs shipped: their crates (the binaries' names come from the manifests).
const PROGRAMS: [&str; 2] = ["game", "editor"];

/// The user manuals, copied into `manuals/`.
const MANUALS: [&str; 2] = ["GAME.md", "EDITOR.md"];

/// The licences of the fonts whose glyphs are in the executables.
const FONT_LICENCES: [&str; 3] = ["OFL-BIZUDGothic.txt", "OFL-NotoSansCJK.txt", "OFL-FusionPixelFont.txt"];

/// The crates checked against Linux: they know no system.
const PORTABLE: [&str; 5] = ["gb-core", "cart-model", "project-model", "i18n", "fonts"];
const LINUX: &str = "x86_64-unknown-linux-gnu";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("targets") => {
            TARGETS.iter().for_each(|t| println!("{} ({})", t.name, t.triple));
            Ok(())
        }
        Some("package") => match TARGETS.iter().find(|t| Some(t.name) == args.get(1).map(String::as_str)) {
            Some(t) => package(t),
            None => Err(format!("package what? The targets are: {}", TARGETS.iter().map(|t| t.name).collect::<Vec<_>>().join(", "))),
        },
        Some("check-portable") => check_portable(),
        _ => Err("usage: cargo xtask targets | package TARGET | check-portable".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn cargo() -> Command {
    let mut c = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.current_dir(root());
    c
}

fn run(mut c: Command) -> Result<(), String> {
    let status = c.status().map_err(|e| format!("cargo: {e}"))?;
    status.success().then_some(()).ok_or_else(|| format!("{c:?} failed"))
}

/// `cargo metadata` for `triple`'s platform.
fn metadata(triple: &str) -> Result<serde_json::Value, String> {
    let out = cargo().args(["metadata", "--format-version", "1", "--filter-platform", triple]).output().map_err(|e| format!("cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo metadata: {}", String::from_utf8_lossy(&out.stderr)));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: {e}"))
}

fn package(t: &Target) -> Result<(), String> {
    let (name, id) = (env!("PRODUCT_NAME"), env!("PRODUCT_ID"));
    let mut build = cargo();
    build.args(["build", "--profile", "dist", "--target", t.triple]);
    for p in PROGRAMS {
        build.args(["-p", p]);
    }
    run(build)?;
    let meta = metadata(t.triple)?;
    let packages = meta["packages"].as_array().ok_or("cargo metadata: no packages")?;
    let version = packages.iter().find(|p| p["name"] == PROGRAMS[0]).and_then(|p| p["version"].as_str()).ok_or("no game package")?.to_string();
    let target_dir = PathBuf::from(meta["target_directory"].as_str().ok_or("cargo metadata: no target directory")?);
    let built = target_dir.join(t.triple).join("dist");
    let suffix = if t.triple.contains("windows") { ".exe" } else { "" };

    let folder = root().join("dist").join(t.name).join(id);
    if folder.exists() {
        std::fs::remove_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    for sub in ["licenses", "manuals"] {
        std::fs::create_dir_all(folder.join(sub)).map_err(|e| format!("{}: {e}", folder.display()))?;
    }

    let mut programs = Vec::new();
    for p in PROGRAMS {
        let bin = packages.iter().find(|q| q["name"] == p).and_then(|q| q["targets"].as_array()?.iter().find(|t| t["kind"][0] == "bin")?["name"].as_str()).ok_or(format!("no binary in {p}"))?;
        let file = format!("{bin}{suffix}");
        let from = built.join(&file);
        check_imports(&from, t)?;
        std::fs::copy(&from, folder.join(&file)).map_err(|e| format!("{}: {e}", from.display()))?;
        programs.push(file);
    }

    let fill = |text: &str| {
        let text = text.replace("{product}", name).replace("{game}", &programs[0]).replace("{editor}", &programs[1]).replace("{source}", env!("PRODUCT_SOURCE"));
        if suffix.is_empty() { text } else { text.replace("\r\n", "\n").replace('\n', "\r\n") }
    };
    for f in ["README.txt", "NO-CARTRIDGE.txt"] {
        let text = std::fs::read_to_string(root().join("packaging").join(f)).map_err(|e| format!("packaging/{f}: {e}"))?;
        std::fs::write(folder.join(f), fill(&text)).map_err(|e| format!("{f}: {e}"))?;
    }
    for (from, to) in [("LICENSE", "LICENSE.txt"), ("LEGAL.md", "LEGAL.md"), ("CHANGELOG.md", "CHANGELOG.md")] {
        let text = std::fs::read_to_string(root().join(from)).map_err(|e| format!("{from}: {e}"))?;
        std::fs::write(folder.join(to), fill(&text)).map_err(|e| format!("{to}: {e}"))?;
    }
    for f in MANUALS {
        let text = std::fs::read_to_string(root().join("docs").join(f)).map_err(|e| format!("docs/{f}: {e}"))?;
        std::fs::write(folder.join("manuals").join(f), fill(&text)).map_err(|e| format!("manuals/{f}: {e}"))?;
    }
    for f in FONT_LICENCES {
        std::fs::copy(root().join("fonts").join(f), folder.join("licenses").join(f)).map_err(|e| format!("fonts/{f}: {e}"))?;
    }
    let crates = licences::third_party(&meta, &PROGRAMS)?;
    std::fs::write(folder.join("licenses").join("THIRD-PARTY.txt"), fill(&crates)).map_err(|e| format!("THIRD-PARTY.txt: {e}"))?;

    let zip = root().join("dist").join(format!("{id}-{version}-{}.zip", t.name));
    zip_folder(&folder, id, &zip)?;
    println!("package: {}", folder.display());
    println!("zip:     {}", zip.display());
    Ok(())
}

/// Stops when `exe` imports a library the target's computers may not have.
fn check_imports(exe: &Path, t: &Target) -> Result<(), String> {
    use object::read::Object;
    let data = std::fs::read(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let file = object::File::parse(&*data).map_err(|e| format!("{}: {e}", exe.display()))?;
    let imports = file.imports().map_err(|e| format!("{}: {e}", exe.display()))?;
    let mut bad: Vec<String> = imports.iter().map(|i| String::from_utf8_lossy(i.library()).to_lowercase()).filter(|l| t.forbidden_imports.iter().any(|f| l.starts_with(f))).collect();
    bad.sort();
    bad.dedup();
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("{} needs {} (the C runtime is not linked in: see .cargo/config.toml)", exe.display(), bad.join(", ")))
    }
}

/// `folder`'s files into `zip`, under a top folder `top`.
fn zip_folder(folder: &Path, top: &str, zip: &Path) -> Result<(), String> {
    let file = std::fs::File::create(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let mut w = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut stack = vec![folder.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
            if e.path().is_dir() {
                stack.push(e.path());
            } else {
                files.push(e.path());
            }
        }
    }
    files.sort();
    for f in files {
        let rel = f.strip_prefix(folder).unwrap().to_string_lossy().replace('\\', "/");
        w.start_file(format!("{top}/{rel}"), options).map_err(|e| e.to_string())?;
        std::io::copy(&mut std::fs::File::open(&f).map_err(|e| format!("{}: {e}", f.display()))?, &mut w).map_err(|e| e.to_string())?;
    }
    w.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn check_portable() -> Result<(), String> {
    let installed = Command::new("rustup").args(["target", "list", "--installed"]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains(LINUX)).unwrap_or(false);
    if !installed {
        return Err(format!("the {LINUX} target is not installed: rustup target add {LINUX}"));
    }
    let mut c = cargo();
    c.args(["check", "--target", LINUX]);
    for p in PORTABLE {
        c.args(["-p", p]);
    }
    run(c)
}
