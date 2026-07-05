use std::fs;

fn main() {
    // -----------------------------------------------------------------------
    // Version — single source of truth via the `release` file.
    // Available in code as env!("LMAHJONG_VERSION").
    // -----------------------------------------------------------------------
    let version = fs::read_to_string("release")
        .expect("Failed to read `release` file")
        .trim()
        .to_string();

    println!("cargo:rustc-env=LMAHJONG_VERSION={}", version);
    println!("cargo:rerun-if-changed=release");

    // -----------------------------------------------------------------------
    // Windows-only: hide the console window and embed .exe metadata / icon.
    //
    // SDL2_LIB_DIR must point to the directory containing SDL2.lib,
    // SDL2_image.lib, SDL2_mixer.lib, SDL2_ttf.lib.
    // package.ps1 sets this automatically; for manual builds set it yourself:
    //   $env:SDL2_LIB_DIR = "C:\path\to\SDL2-devel-2.x.x-VC\lib\x64"
    // -----------------------------------------------------------------------
    #[cfg(target_os = "windows")]
    windows_resources(&version);
}

/// Configures the Windows build:
///
/// 1. Sets the linker subsystem to `windows` so no console window pops up
///    when the player launches the .exe from Explorer or a shortcut.
///
/// 2. Uses `winres` to embed a VERSIONINFO resource (product name, version,
///    copyright) and the application icon (assets/icon.ico).
///    The icon appears in Explorer, the taskbar, Alt-Tab, and the Start Menu.
///
/// This function is compiled only on Windows (`#[cfg(target_os = "windows")]`),
/// so Linux and macOS builds are completely unaffected.
#[cfg(target_os = "windows")]
fn windows_resources(version: &str) {
    // Tell the linker to use the GUI subsystem — suppresses the console window.
    println!("cargo:rustc-link-arg-bins=/SUBSYSTEM:WINDOWS");
    // SDL2 provides the SDL_main shim that maps WinMain → main automatically.
    println!("cargo:rustc-link-arg-bins=/ENTRY:mainCRTStartup");

    // Point winres at the Windows SDK rc.exe if it is not on PATH.
    // winres tries windres/llvm-rc first; rc.exe is the MSVC resource compiler.
    let mut res = winres::WindowsResource::new();

    // Find rc.exe in the Windows SDK and tell winres where to find it.
    // winres calls the tool named "windres" by default; on MSVC we redirect it
    // to the Windows SDK rc.exe via set_toolkit_path.
    let sdk_bin = std::path::Path::new(
        r"C:\Program Files (x86)\Windows Kits\10\bin"
    );
    if sdk_bin.exists() {
        if let Ok(entries) = std::fs::read_dir(sdk_bin) {
            let mut versions: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            // Sort descending so newest SDK version is first
            versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            for entry in versions {
                let rc = entry.path().join("x64").join("rc.exe");
                if rc.exists() {
                    // set_toolkit_path tells winres which directory contains rc.exe
                    res.set_toolkit_path(
                        entry.path().join("x64").to_str().unwrap_or("")
                    );
                    break;
                }
            }
        }
    }

    res.set("ProductName", "LMahjong");
    res.set("FileDescription", "A Tux-themed Mahjong solitaire game");
    res.set("LegalCopyright", "GPL-3.0-or-later");
    res.set("FileVersion", version);
    res.set("ProductVersion", version);

    // Prefer the pre-generated .ico (multi-size: 16,32,48,64,128,256).
    // Fall back to the .png if the .ico hasn't been generated yet.
    let icon_ico = "assets/icon.ico";
    let icon_png = "assets/icon.png";
    if std::path::Path::new(icon_ico).exists() {
        res.set_icon(icon_ico);
        println!("cargo:rerun-if-changed={}", icon_ico);
    } else if std::path::Path::new(icon_png).exists() {
        res.set_icon(icon_png);
        println!("cargo:rerun-if-changed={}", icon_png);
    } else {
        eprintln!("build.rs: no icon found, skipping icon embedding.");
    }

    if let Err(e) = res.compile() {
        eprintln!("build.rs: winres compile warning (non-fatal): {}", e);
    }
}
