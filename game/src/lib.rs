//! Propósito del archivo:
//! Punto de entrada principal de la librería móvil `horizonte_latam`.
//! En Android expone `android_main(AndroidApp)` usando `android-activity` y `eframe`
//! con renderizador `wgpu` (sin ninguna capa ni fallback en Kotlin).
//! En escritorio/pruebas provee `run_desktop()`.

pub mod ai;
pub mod cache;
pub mod engine;
pub mod lua_bridge;
pub mod map;
pub mod models;
pub mod ui;

use crate::ui::HorizonteLatamApp;

fn build_wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(ref mut setup) = wgpu_options.wgpu_setup {
        setup.power_preference = eframe::egui_wgpu::wgpu::PowerPreference::HighPerformance;
        setup.instance_descriptor.backends = eframe::egui_wgpu::wgpu::Backends::VULKAN
            | eframe::egui_wgpu::wgpu::Backends::GL;
        setup.device_descriptor = std::sync::Arc::new(|adapter| {
            let mut required_features = eframe::egui_wgpu::wgpu::Features::empty();
            if adapter
                .features()
                .contains(eframe::egui_wgpu::wgpu::Features::PIPELINE_CACHE)
            {
                required_features |= eframe::egui_wgpu::wgpu::Features::PIPELINE_CACHE;
            }
            eframe::egui_wgpu::wgpu::DeviceDescriptor {
                label: Some("horizonte_latam_gpu"),
                required_features,
                required_limits: adapter.limits(),
                ..Default::default()
            }
        });
    }
    wgpu_options
}

/// Punto de entrada nativo para Android (`NativeActivity` en 32-bit y 64-bit).
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("HorizonteLatam"),
    );

    let android_cache_dir = app
        .internal_data_path()
        .and_then(|files_dir| files_dir.parent().map(|p| p.join("cache")));
    cache::init_cache_dir(android_cache_dir);

    let options = eframe::NativeOptions {
        android_app: Some(app),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: build_wgpu_options(),
        viewport: egui::ViewportBuilder::default().with_fullscreen(true),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "Horizonte Latam",
        options,
        Box::new(|cc| Ok(Box::new(HorizonteLatamApp::new(cc)))),
    ) {
        log::error!("Error fatal ejecutando interfaz Rust/wgpu en Android: {err}");
    }
}

/// Lanzador opcional para probar el juego en Linux/escritorio sin emulador Android.
#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> eframe::Result<()> {
    cache::init_cache_dir(None);

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: build_wgpu_options(),
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Horizonte Latam — Estrategia 3D (Rust + wgpu + Lua 5.4)"),
        ..Default::default()
    };

    eframe::run_native(
        "Horizonte Latam",
        options,
        Box::new(|cc| Ok(Box::new(HorizonteLatamApp::new(cc)))),
    )
}
