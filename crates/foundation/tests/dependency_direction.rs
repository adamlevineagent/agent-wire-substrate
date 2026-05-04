use std::fs;
use std::path::Path;

#[test]
fn foundation_source_does_not_import_forbidden_dependents() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0usize;

    for entry in fs::read_dir(root).expect("read foundation src") {
        let entry = entry.expect("dir entry");
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        checked += 1;
        let source = fs::read_to_string(entry.path()).expect("read source");
        for forbidden in [
            "agent_wire_node",
            "agent_wire_compute_market",
            "agent_wire_storage_market",
            "agent_wire_relay_market",
            "agent_wire_transport_cloudflare",
            "tauri",
            "cloudflare",
            "pyramid",
        ] {
            assert!(
                !source.contains(forbidden),
                "foundation source must not depend on {forbidden}"
            );
        }
    }

    assert!(checked >= 6, "expected foundation modules to be present");
}
