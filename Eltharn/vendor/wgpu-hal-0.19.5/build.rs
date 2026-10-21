use std::env;

fn feature(name: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_{name}")).is_some()
}

fn main() {
    let architecture =
        env::var("CARGO_CFG_TARGET_ARCH").expect("Cargo supplies target architecture");
    let operating_system = env::var("CARGO_CFG_TARGET_OS").expect("Cargo supplies target OS");
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let native = architecture != "wasm32";
    let gles = feature("GLES");
    let atomics = target_features.split(',').any(|name| name == "atomics");

    for (name, enabled) in [
        ("native", native),
        (
            "send_sync",
            native || (feature("FRAGILE_SEND_SYNC_NON_ATOMIC_WASM") && !atomics),
        ),
        ("webgl", !native && operating_system != "emscripten" && gles),
        ("Emscripten", operating_system == "emscripten" && gles),
        ("dx12", operating_system == "windows" && feature("DX12")),
        ("gles", gles),
        (
            "metal",
            matches!(operating_system.as_str(), "ios" | "macos") && feature("METAL"),
        ),
        ("vulkan", native && feature("VULKAN")),
    ] {
        println!("cargo:rustc-check-cfg=cfg({name})");
        if enabled {
            println!("cargo:rustc-cfg={name}");
        }
    }
}
