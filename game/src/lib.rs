//! Propósito del archivo:
//! Punto de entrada principal de la librería móvil `horizonte_latam`.
//! En Android expone `android_main(AndroidApp)` usando `android-activity` y `eframe`
//! (sin ninguna capa ni fallback en Kotlin). En escritorio/pruebas provee `run_desktop()`.

pub mod ai;
pub mod engine;
pub mod lua_bridge;
pub mod map;
pub mod models;
pub mod ui;

use crate::ui::HorizonteLatamApp;

/// Punto de entrada nativo para Android (`NativeActivity` en 32-bit y 64-bit).
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("HorizonteLatam"),
    );

    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "Horizonte Latam",
        options,
        Box::new(|_cc| Ok(Box::new(HorizonteLatamApp::new()))),
    ) {
        log::error!("Error fatal ejecutando interfaz Rust en Android: {err}");
    }
}

/// Lanzador opcional para probar el juego en Linux/escritorio sin emulador Android.
#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Horizonte Latam — Estrategia (Rust + Lua 5.4)"),
        ..Default::default()
    };

    eframe::run_native(
        "Horizonte Latam",
        options,
        Box::new(|_cc| Ok(Box::new(HorizonteLatamApp::new()))),
    )
}
