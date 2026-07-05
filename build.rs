use std::fs;

fn main() {
    // Read version from the `release` file at the project root.
    // This becomes the single source of truth for the application version.
    let version = fs::read_to_string("release")
        .expect("Failed to read `release` file")
        .trim()
        .to_string();

    // Expose it as an environment variable available via env!("LMAHJONG_VERSION") in code.
    println!("cargo:rustc-env=LMAHJONG_VERSION={}", version);

    // Re-run this build script if the release file changes.
    println!("cargo:rerun-if-changed=release");
}
