#![windows_subsystem = "windows"]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--print-backends") {
        println!(
            "{}",
            serde_json::to_string(&vec![el_protocol_1_21_11::backend_manifest()]).unwrap()
        );
        return;
    }
    let mode = if args.first().is_some_and(|arg| arg == "--mode") {
        args.get(1).map(String::as_str).unwrap_or("")
    } else if args.iter().any(|arg| arg == "--eltharnPrintProtocolInfo") {
        "game"
    } else {
        "gate"
    };
    match mode {
        "game" => {
            el_client::register_builtin_backend(el_protocol_1_21_11::backend_manifest())
                .expect("compatible built-in backend");
            el_client::run_desktop_client();
        }
        "backend" => {
            let requested = args
                .iter()
                .position(|arg| arg == "--protocol")
                .and_then(|index| args.get(index + 1));
            if requested.map(String::as_str) != Some(el_protocol_1_21_11::VANILLA_VERSION_NAME) {
                eprintln!("The requested built-in protocol is unavailable");
                std::process::exit(2);
            }
            el_protocol_1_21_11::run_stdio_backend();
        }
        "gate" => {
            let executable = std::env::current_exe().expect("resolve desktop executable");
            let backend = el_protocol_1_21_11::backend_manifest();
            let manifest = el_launcher_core::ProtocolBackendManifest {
                ipc_version: backend.ipc_version,
                backend_name: backend.backend_name,
                minecraft_version: backend.minecraft_version,
                protocol_version: backend.protocol_version,
                protocol_source: backend.protocol_source,
                executable: "eltharn".to_owned(),
                assets_compatibility: backend.assets_compatibility,
            };
            el_gate::run_gate(Some(el_gate::BundledRuntime {
                executable,
                manifest,
            }));
        }
        _ => {
            eprintln!("Usage: eltharn [--mode gate|game|backend] [options]");
            std::process::exit(2);
        }
    }
}
