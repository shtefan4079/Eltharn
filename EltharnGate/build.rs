fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    println!("cargo:rerun-if-changed=src/icon_256.png");
    println!("cargo:rerun-if-changed=resources.rc");
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let png = std::fs::read(root.join("src/icon_256.png")).expect("read launcher icon");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n") && png.len() >= 24);
    assert_eq!(&png[16..24], &[0, 0, 1, 0, 0, 0, 1, 0], "256x256 icon");

    let mut ico = Vec::with_capacity(22 + png.len());
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    ico.extend_from_slice(&[0, 0, 0, 0, 1, 0, 32, 0]);
    ico.extend_from_slice(&u32::try_from(png.len()).expect("icon size").to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&png);
    let icon_path = out.join("icon.ico");
    std::fs::write(&icon_path, ico).expect("write Windows icon");

    let icon_path = icon_path.to_string_lossy().replace('\\', "/");
    let rc = std::fs::read_to_string(root.join("resources.rc"))
        .expect("read Windows resource template")
        .replace("\"icon.ico\"", &format!("\"{icon_path}\""));
    let resource_path = out.join("resources.rc");
    std::fs::write(&resource_path, rc).expect("write Windows resources");
    embed_resource::compile(&resource_path, embed_resource::NONE);
}
