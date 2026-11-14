#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;
mod dsp;
mod hotkeys;
mod i18n;
mod presets;
mod routing;
mod ui;
fn diagnostic_dir() -> std::path::PathBuf {
    std::env::args()
        .find_map(|arg| {
            arg.strip_prefix("--output-dir=")
                .map(std::path::PathBuf::from)
        })
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("diagnostics")
        })
}
fn main() -> eframe::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|s| s == "--route-watchdog") {
        if let (Some(path), Some(token), Some(pid)) = (args.get(2), args.get(3), args.get(4)) {
            if let Ok(pid) = pid.to_string_lossy().parse() {
                routing::watchdog(std::path::Path::new(path), &token.to_string_lossy(), pid);
            }
        }
        return Ok(());
    }
    if args.get(1).is_some_and(|s| s == "--route-diagnostics") {
        let dir = diagnostic_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("routing-diagnostics.json");
        if let Err(e) = routing::diagnose(&path) {
            let _ = std::fs::write(path, format!("FAIL: {e}"));
        }
        return Ok(());
    }
    if std::env::args().any(|s| s == "--audio-test") {
        let dir = diagnostic_dir();
        let _ = std::fs::create_dir_all(&dir);
        audio::smoke_test(&dir);
        return Ok(());
    }
    if std::env::args().any(|s| s == "--self-test") {
        return ui::self_test(&diagnostic_dir());
    }
    // Offline screenshots use a private profile and never acquire the live
    // audio instance. They cannot start processing or change system routing.
    let screenshot_mode = std::env::args().any(|arg| arg.starts_with("--screenshot="))
        && !std::env::args().any(|arg| arg == "--audio-smoke");
    let _instance = if screenshot_mode {
        None
    } else {
        Some(match routing::Instance::acquire() {
            Ok(instance) => instance,
            Err(e) => {
                use windows::{
                    core::{w, PCWSTR},
                    Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK},
                };
                let text: Vec<u16> = e.encode_utf16().chain(Some(0)).collect();
                unsafe {
                    MessageBoxW(None, PCWSTR(text.as_ptr()), w!("VESPER"), MB_OK);
                }
                return Ok(());
            }
        })
    };
    let icon = image::load_from_memory(include_bytes!("../assets/vesper.png"))
        .unwrap()
        .into_rgba8();
    let (width, height) = icon.dimensions();
    let qa_size = if screenshot_mode {
        std::env::args().find_map(|arg| {
            let (width, height) = arg.strip_prefix("--qa-size=")?.split_once('x')?;
            Some([
                width.parse::<f32>().ok()?.clamp(1100., 1920.),
                height.parse::<f32>().ok()?.clamp(700., 1200.),
            ])
        })
    } else {
        None
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(qa_size.unwrap_or([1380., 740.]))
            .with_position([24., 12.])
            .with_min_inner_size([1100., 700.])
            .with_title("VESPER — Voice Studio")
            .with_icon(eframe::egui::IconData {
                rgba: icon.into_raw(),
                width,
                height,
            }),
        multisampling: 0,
        ..Default::default()
    };
    eframe::run_native(
        "VESPER",
        options,
        Box::new(|cc| Ok(Box::new(ui::Studio::new(cc)))),
    )
}
