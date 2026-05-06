use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=AGENT_WIRE_CLOUDFLARED_BUNDLE_SOURCE");
    let Ok(source) = env::var("AGENT_WIRE_CLOUDFLARED_BUNDLE_SOURCE") else {
        return;
    };
    let source = source.trim();
    if source.is_empty() {
        return;
    }

    let source = PathBuf::from(source);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let bundle_dir = out_dir.join("cloudflared-bundle");
    fs::create_dir_all(&bundle_dir).expect("create cloudflared bundle dir");
    let binary_name = if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        "cloudflared.exe"
    } else {
        "cloudflared"
    };
    let destination = bundle_dir.join(binary_name);
    fs::copy(&source, &destination).expect("copy cloudflared bundle source");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o755))
            .expect("chmod cloudflared bundle");
    }

    println!(
        "cargo:rustc-env=AGENT_WIRE_BUNDLED_CLOUDFLARED_PATH={}",
        destination.display()
    );
}
