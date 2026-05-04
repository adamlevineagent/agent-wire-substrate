use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn foundation_has_no_forbidden_boundary_references() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![manifest_dir.join("Cargo.toml")];
    collect_rs_files(&manifest_dir.join("src"), &mut files);
    let build_script = manifest_dir.join("build.rs");
    if build_script.exists() {
        files.push(build_script);
    }

    let forbidden = [
        "pyramid",
        "tauri",
        "cloudflare",
        "compute-market",
        "compute_market",
        "storage-market",
        "storage_market",
        "relay-market",
        "relay_market",
        "agent-wire-substrate-node",
        "agent_wire_substrate_node",
    ];

    let mut violations = Vec::new();
    for file in files {
        if file
            .file_name()
            .is_some_and(|name| name == "dependency_guard.rs")
        {
            continue;
        }
        let text = fs::read_to_string(&file).expect("read guard target");
        let lower = text.to_ascii_lowercase();
        for token in forbidden {
            if allowed_foundation_owned_token(&file, token) {
                continue;
            }
            if lower.contains(token) {
                violations.push(format!(
                    "{} contains forbidden token `{}`",
                    file.display(),
                    token
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "foundation boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn foundation_has_no_forbidden_transitive_dependencies() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("foundation crate sits under crates/");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(workspace_dir)
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse cargo metadata");
    let package_by_id = metadata["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .map(|package| {
            (
                package["id"].as_str().expect("package id").to_owned(),
                package["name"].as_str().expect("package name").to_owned(),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let foundation_id = package_by_id
        .iter()
        .find_map(|(id, name)| (name == "agent-wire-foundation").then_some(id.clone()))
        .expect("agent-wire-foundation package");
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .expect("metadata resolve nodes");
    let deps_by_id = nodes
        .iter()
        .map(|node| {
            let id = node["id"].as_str().expect("node id").to_owned();
            let deps = node["deps"]
                .as_array()
                .expect("node deps")
                .iter()
                .map(|dep| dep["pkg"].as_str().expect("dep package id").to_owned())
                .collect::<Vec<_>>();
            (id, deps)
        })
        .collect::<std::collections::HashMap<_, _>>();
    let forbidden = [
        "agent-wire-transport-cloudflare",
        "agent-wire-compute-market",
        "agent-wire-storage-market",
        "agent-wire-relay-market",
        "agent-wire-substrate-node",
        "tauri",
    ];
    let mut stack = deps_by_id.get(&foundation_id).cloned().unwrap_or_default();
    let mut visited = std::collections::HashSet::new();
    let mut violations = Vec::new();
    while let Some(package_id) = stack.pop() {
        if !visited.insert(package_id.clone()) {
            continue;
        }
        let name = package_by_id
            .get(&package_id)
            .expect("dependency package exists");
        if forbidden.contains(&name.as_str()) {
            violations.push(name.clone());
        }
        stack.extend(deps_by_id.get(&package_id).cloned().unwrap_or_default());
    }

    assert!(
        violations.is_empty(),
        "foundation forbidden transitive dependencies:\n{}",
        violations.join("\n")
    );
}

fn allowed_foundation_owned_token(file: &Path, token: &str) -> bool {
    file.file_name().is_some_and(|name| name == "vocabulary.rs")
        && matches!(
            token,
            "cloudflare" | "compute-market" | "storage-market" | "relay-market"
        )
}

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read src directory") {
        let entry = entry.expect("read src entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}
