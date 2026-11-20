//! The licences of the Rust crates compiled into the programs, from `cargo metadata`: every
//! crate reached from them through normal dependencies (not build or dev ones), with its
//! licence and the text of its licence files.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The MIT licence, for a crate that declares it without shipping its text.
const MIT: &str = "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.\n";

/// A crate's licence files: LICENSE*, LICENCE*, COPYING*, NOTICE* in its folder.
fn licence_files(dir: &Path) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let upper = name.to_uppercase();
            ["LICENSE", "LICENCE", "COPYING", "NOTICE"].iter().any(|p| upper.starts_with(p)).then(|| std::fs::read_to_string(e.path()).ok().map(|t| (name, t)))?
        })
        .collect();
    v.sort();
    v
}

/// The text of `licenses/THIRD-PARTY.txt` for the crates of `programs`.
pub fn third_party(meta: &Value, programs: &[&str]) -> Result<String, String> {
    let packages: BTreeMap<&str, &Value> = meta["packages"].as_array().ok_or("cargo metadata: no packages")?.iter().filter_map(|p| Some((p["id"].as_str()?, p))).collect();
    let members: BTreeSet<&str> = meta["workspace_members"].as_array().ok_or("cargo metadata: no members")?.iter().filter_map(Value::as_str).collect();
    let nodes: BTreeMap<&str, &Value> = meta["resolve"]["nodes"].as_array().ok_or("cargo metadata: no resolve")?.iter().filter_map(|n| Some((n["id"].as_str()?, n))).collect();
    let mut todo: Vec<&str> = packages.iter().filter(|(_, p)| programs.iter().any(|&g| p["name"] == g)).map(|(&id, _)| id).collect();
    let mut seen = BTreeSet::new();
    while let Some(id) = todo.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(node) = nodes.get(id) else { continue };
        for dep in node["deps"].as_array().into_iter().flatten() {
            let normal = dep["dep_kinds"].as_array().into_iter().flatten().any(|k| k["kind"].is_null());
            if let (true, Some(d)) = (normal, dep["pkg"].as_str()) {
                todo.push(d);
            }
        }
    }
    let mut crates: Vec<&Value> = seen.iter().filter(|id| !members.contains(*id)).filter_map(|id| packages.get(id).copied()).collect();
    crates.sort_by_key(|p| (p["name"].as_str().unwrap_or("").to_lowercase(), p["version"].as_str().unwrap_or("").to_string()));

    let mut out = format!("{{product}} includes these Rust crates, each under its own licence.\n\n{} crates:\n", crates.len());
    for p in &crates {
        out += &format!("  {} {} ({})\n", p["name"].as_str().unwrap_or("?"), p["version"].as_str().unwrap_or("?"), p["license"].as_str().unwrap_or("licence not declared"));
    }
    for p in &crates {
        let (name, version) = (p["name"].as_str().unwrap_or("?"), p["version"].as_str().unwrap_or("?"));
        let licence = p["license"].as_str().unwrap_or("");
        out += &format!("\n\n{}\n{name} {version}\nLicence: {}\n", "=".repeat(78), if licence.is_empty() { "not declared" } else { licence });
        if let Some(repo) = p["repository"].as_str() {
            out += &format!("Source: {repo}\n");
        }
        let dir = Path::new(p["manifest_path"].as_str().unwrap_or("")).parent().unwrap_or(Path::new("."));
        let files = licence_files(dir);
        if files.is_empty() {
            if licence.contains("MIT") {
                let authors: Vec<&str> = p["authors"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
                out += &format!("\nCopyright (c) {}\n\n{MIT}", if authors.is_empty() { format!("the {name} authors") } else { authors.join(", ") });
            } else {
                eprintln!("warning: {name} {version} ships no licence file ({})", if licence.is_empty() { "no licence declared" } else { licence });
                out += "\n(no licence file in the crate; see its source)\n";
            }
        }
        for (file, text) in files {
            out += &format!("\n--- {file} ---\n\n{}\n", text.trim_end());
        }
    }
    Ok(out)
}
