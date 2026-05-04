use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn foundation_has_no_forbidden_boundary_references() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![manifest_dir.join("Cargo.toml")];
    collect_rs_files(&manifest_dir.join("src"), &mut files);

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
        if file.ends_with("dependency_guard.rs") {
            continue;
        }
        let text = fs::read_to_string(&file).expect("read guard target");
        let lower = text.to_ascii_lowercase();
        for token in forbidden {
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
