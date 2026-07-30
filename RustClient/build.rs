//! Inject a short build stamp so the window title can prove which binary is running.
fn main() {
    // Rebuild when sources change is cargo's job; stamp changes every compile.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=OHOL_BUILD_STAMP={stamp}");
    // Force rebuild of bins that use env!("OHOL_BUILD_STAMP") when this file changes.
    println!("cargo:rerun-if-changed=build.rs");
}
