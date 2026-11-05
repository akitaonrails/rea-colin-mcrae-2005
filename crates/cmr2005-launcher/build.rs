//! The Windows executable's version information, icon and manifest
//! (rea-launcher-build): a named, versioned program with the launcher's
//! icon, instead of an anonymous one that antivirus heuristics distrust.
fn main() {
    rea_launcher_build::Resources::new(
        "Colin McRae Rally 2005 Launcher",
        "Launcher for Colin McRae Rally 2005 on modern systems",
    )
    .company("AkitaOnRails")
    .file_name("CMR2005Launcher.exe")
    .icon("../../packaging/appimage/cmr2005-launcher.png")
    .embed()
    .unwrap();
}
