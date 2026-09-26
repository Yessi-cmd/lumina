use tauri_build::{Attributes, WindowsAttributes};

fn main() {
    // Run elevated: see app.manifest.
    let manifest = include_str!("app.manifest");
    let windows = WindowsAttributes::new().app_manifest(manifest);
    tauri_build::try_build(Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
}
