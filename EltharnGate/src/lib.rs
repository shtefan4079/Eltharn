mod blur;
mod bundled;
mod icons;
mod install;
mod launcher;
mod style;
pub use bundled::{BundledRuntime, bundled_runtime};
use launcher::EltharnGateApp;
use launcher::paths::EltharnGatePaths;
fn theme(_state: &EltharnGateApp) -> iced::Theme {
    iced::Theme::custom(
        "EltharnGateTheme".to_string(),
        iced::theme::Palette {
            primary: style::ACCENT_BLUE,
            background: if std::env::var_os("ELTHARN_GATE_UI_CAPTURE").is_some() {
                iced::Color::from_rgb(0.055, 0.065, 0.085)
            } else {
                iced::Color::TRANSPARENT
            },
            ..iced::Theme::Dark.palette()
        },
    )
}
fn title(_state: &EltharnGateApp) -> String {
    "Eltharn Gate".to_string()
}
fn window_settings(transparent: bool, blur: bool) -> iced::window::Settings {
    let icon = {
        const ICON_RGBA: &[u8] = include_bytes!("icon_rgba.bin");
        iced::window::icon::from_rgba(ICON_RGBA.to_vec(), 256, 256).ok()
    };
    #[allow(unused_mut)]
    let mut settings = iced::window::Settings {
        size: (960.0, 620.0).into(),
        min_size: Some((800.0_f32, 500.0_f32).into()),
        position: iced::window::Position::Centered,
        resizable: true,
        decorations: false,
        transparent,
        blur,
        icon,
        ..Default::default()
    };
    if std::env::var_os("ELTHARN_GATE_UI_CAPTURE").is_some() {
        if let Ok(size) = std::env::var("ELTHARN_GATE_UI_CAPTURE_SIZE") {
            if let Some((width, height)) = size.split_once('x') {
                if let (Ok(width), Ok(height)) = (width.parse::<u16>(), height.parse::<u16>()) {
                    settings.size = (
                        f32::from(width.clamp(800, 1920)),
                        f32::from(height.clamp(500, 1080)),
                    )
                        .into();
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.application_id = "eltharn_gate".to_string();
    }
    settings
}
#[cfg(target_os = "linux")]
fn ensure_desktop_integration() {
    fn write_if_changed(path: &std::path::Path, contents: &[u8]) -> bool {
        if std::fs::read(path).ok().as_deref() == Some(contents) {
            return false;
        }
        std::fs::write(path, contents).is_ok()
    }
    if std::env::var_os("ELTHARN_GATE_DATA_DIR").is_some()
        || std::env::var_os("RONNER_DATA_DIR").is_some()
    {
        return;
    }
    if std::env::var_os("ELTHARN_GATE_NO_DESKTOP_INTEGRATION").is_some() {
        return;
    }
    if let Some(home) = dirs::home_dir() {
        let app_dir = home.join(".local/share/applications");
        let desktop_file = app_dir.join("eltharn_gate.desktop");
        let icon_dir_256 = home.join(".local/share/icons/hicolor/256x256/apps");
        let icon_file_256 = icon_dir_256.join("eltharn_gate.png");
        let icon_dir_svg = home.join(".local/share/icons/hicolor/scalable/apps");
        let icon_file_svg = icon_dir_svg.join("eltharn_gate.svg");
        const ICON_256_PNG: &[u8] = include_bytes!("icon_256.png");
        let mut changed = false;
        let _ = std::fs::create_dir_all(&icon_dir_svg);
        changed |= write_if_changed(&icon_file_svg, icons::SVG_LOGO.as_bytes());
        let _ = std::fs::create_dir_all(&icon_dir_256);
        changed |= write_if_changed(&icon_file_256, ICON_256_PNG);
        if let Ok(entries) = std::fs::read_dir(home.join(".local/share/icons")) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let apps_svg = p.join("apps/el_gate.svg");
                    if apps_svg.exists() {
                        changed |= write_if_changed(&apps_svg, icons::SVG_LOGO.as_bytes());
                    }
                    let apps_png = p.join("apps/el_gate.png");
                    if apps_png.exists() {
                        changed |= write_if_changed(&apps_png, ICON_256_PNG);
                    }
                }
            }
        }
        let _ = std::fs::create_dir_all(&app_dir);
        let bin_path = if let Ok(current_exe) = std::env::current_exe() {
            current_exe.display().to_string()
        } else {
            home.join(".local/bin/eltharn_gate").display().to_string()
        };
        let exec = launcher::utils::desktop_exec_quote(&bin_path);
        let desktop_entry = format!(
            "[Desktop Entry]\n\
             Name=Eltharn Gate\n\
             Comment=Eltharn Modern Game Launcher\n\
             Exec={exec}\n\
             Icon=eltharn_gate\n\
             Terminal=false\n\
             Type=Application\n\
             Categories=Game;Application;\n\
             StartupWMClass=eltharn_gate\n\
             StartupNotify=true\n\
             Keywords=minecraft;eltharn;launcher;\n"
        );
        changed |= write_if_changed(&desktop_file, desktop_entry.as_bytes());
        if !changed {
            return;
        }
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&app_dir)
            .output();
        let _ = std::process::Command::new("gtk-update-icon-cache")
            .args(["-f", "-t"])
            .arg(home.join(".local/share/icons/hicolor"))
            .output();
        let _ = std::process::Command::new("kbuildsycoca6")
            .arg("--noincremental")
            .output();
        let _ = std::process::Command::new("kbuildsycoca5")
            .arg("--noincremental")
            .output();
    }
}
pub fn run_gate(runtime: Option<BundledRuntime>) {
    if let Some(runtime) = runtime {
        bundled::register(runtime);
    }
    let capture_mode = std::env::var_os("ELTHARN_GATE_UI_CAPTURE").is_some();
    if capture_mode {
        if let Err(error) = launcher::ui_capture::Capture::from_env() {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
    let mut arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|arg| arg == "--mode")
        && arguments.get(1).is_some_and(|arg| arg == "gate")
    {
        arguments.drain(..2);
    }
    if arguments.first().is_some_and(|arg| arg == "--install") {
        if arguments.len() != 1 {
            eprintln!("Usage: eltharn --install (ELTHARN_GATE_DATA_DIR selects the destination)");
            std::process::exit(2);
        }
        match install::install() {
            Ok(path) => println!("Installed {}", path.display()),
            Err(error) => {
                eprintln!("Installation failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if arguments
        .first()
        .is_some_and(|arg| arg == "--verify-installation")
    {
        let paths = EltharnGatePaths::discover();
        if arguments.len() != 1 {
            eprintln!("Usage: eltharn --verify-installation");
            std::process::exit(2);
        }
        match launcher::executor::verify_builtin_installation(&paths) {
            Ok(()) => println!("Desktop installation verified"),
            Err(error) => {
                eprintln!("Installation verification failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if arguments
        .first()
        .is_some_and(|arg| arg == "--prepare-assets")
    {
        if arguments.len() != 2 {
            eprintln!("Usage: eltharn_gate --prepare-assets <asset-compatibility-version>");
            std::process::exit(2);
        }
        let paths = EltharnGatePaths::discover();
        let result = tokio::runtime::Runtime::new()
            .map_err(|error| error.to_string())
            .and_then(|runtime| {
                runtime.block_on(launcher::downloader::download_version_assets(
                    &paths,
                    &arguments[1],
                    None,
                ))
            });
        match result {
            Ok(report) if report.is_valid => {
                println!(
                    "{}",
                    launcher::downloader::prepared_manifest_path(&paths, &arguments[1]).display()
                );
                return;
            }
            Ok(report) => {
                eprintln!(
                    "Asset preparation incomplete: {}",
                    report.details.join("; ")
                )
            }
            Err(error) => eprintln!("Asset preparation failed: {error}"),
        }
        std::process::exit(1);
    }
    if arguments.first().is_some_and(|arg| arg == "--record-pair") {
        if arguments.len() != 2 {
            eprintln!("{}", launcher::executor::record_pair_cli_usage());
            std::process::exit(2);
        }
        let paths = EltharnGatePaths::discover();
        match launcher::executor::record_pair_for_protocol(&paths, &arguments[1]) {
            Ok(()) => {
                println!("Recorded launch pair for {}", arguments[1]);
                return;
            }
            Err(error) => eprintln!("Record launch pair failed: {error}"),
        }
        std::process::exit(1);
    }
    #[cfg(target_os = "linux")]
    if !capture_mode {
        ensure_desktop_integration();
    }
    let paths = EltharnGatePaths::discover();
    if !capture_mode {
        if let Err(error) = launcher::executor::initialize_builtin_installation(&paths) {
            eprintln!("Desktop installation could not be initialized: {error}");
            std::process::exit(1);
        }
    }
    let default_hook = std::panic::take_hook();
    let crash_log_path = paths.crash_log_file();
    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => s.as_str(),
                None => "Unknown panic payload",
            },
        };
        let location = info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let crash_content = format!(
            "Eltharn Gate Panic Report\nTimestamp: {:?}\nLocation: {}\nMessage: {}\n",
            std::time::SystemTime::now(),
            location,
            msg
        );
        let _ = std::fs::write(&crash_log_path, crash_content);
    }));
    #[cfg(target_os = "linux")]
    if !capture_mode {
        blur::clear_legacy_kwin_rules_blur();
    }
    let run_res = iced::application(
        EltharnGateApp::new,
        EltharnGateApp::update,
        EltharnGateApp::view,
    )
    .title(title)
    .window(window_settings(!capture_mode, !capture_mode))
    .subscription(EltharnGateApp::subscription)
    .theme(theme)
    .run();
    if let Err(e1) = run_res {
        eprintln!(
            "[Eltharn Gate] Stage 1 failed ({e1:?}), attempting Stage 2 (transparent without blur)..."
        );
        let run_res2 = iced::application(
            EltharnGateApp::new,
            EltharnGateApp::update,
            EltharnGateApp::view,
        )
        .title(title)
        .window(window_settings(true, false))
        .subscription(EltharnGateApp::subscription)
        .theme(theme)
        .run();
        if let Err(e2) = run_res2 {
            eprintln!(
                "[Eltharn Gate] Stage 2 failed ({e2:?}), attempting Stage 3 (opaque fallback)..."
            );
            let run_res3 = iced::application(
                EltharnGateApp::new,
                EltharnGateApp::update,
                EltharnGateApp::view,
            )
            .title(title)
            .window(window_settings(false, false))
            .subscription(EltharnGateApp::subscription)
            .theme(theme)
            .run();
            if let Err(e3) = run_res3 {
                eprintln!("[Eltharn Gate] All launch stages failed: {e3:?}");
            }
        }
    }
}
