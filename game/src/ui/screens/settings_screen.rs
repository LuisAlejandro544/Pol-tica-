//! Propósito del archivo:
//! Pantalla dedicada de Opciones y Estado del Motor (`Settings`) estilo Sala de Mando.
//! Permite cambiar dificultad mediante botones físicos, inspeccionar la GPU 3D Vulkan/GLES
//! activa y navegar entre mapa, cambio de nación o menú principal.

use crate::models::{Difficulty, GameState};
use crate::ui::theme::{
    card_frame, draw_section_header, game_button, panel_frame, well_frame, GameIcon, ACCENT_CYAN,
    ACCENT_EMERALD, ACCENT_GOLD, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{RichText, Ui, Vec2};

pub enum SettingsAction {
    None,
    ReturnToMap,
    ReturnToMainMenu,
    RestartSelectCountry,
}

pub fn render_settings_screen(
    ui: &mut Ui,
    state: &mut GameState,
    gpu_info: &str,
    target_format: eframe::egui_wgpu::wgpu::TextureFormat,
) -> SettingsAction {
    let mut action = SettingsAction::None;

    egui::ScrollArea::vertical()
        .id_salt("settings_screen_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            panel_frame(true).inner_margin(14.0).show(ui, |ui| {
                draw_section_header(
                    ui,
                    GameIcon::Settings,
                    "OPCIONES DE CAMPAÑA Y MOTOR",
                    ACCENT_GOLD,
                );

                card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                    ui.label(
                        RichText::new("DIFICULTAD DE LA IA LOCAL (RUST + RAYON)")
                            .strong()
                            .size(12.5)
                            .color(ACCENT_CYAN),
                    );
                    ui.add_space(6.0);
                    let seg_w = ((ui.available_width() - 16.0) / 3.0).clamp(95.0, 170.0);
                    ui.horizontal(|ui| {
                        for (diff, lbl) in [
                            (Difficulty::Normal, "Normal"),
                            (Difficulty::Desafiante, "Desafiante"),
                            (Difficulty::Realista, "Realista"),
                        ] {
                            let active = state.difficulty == diff;
                            if game_button(
                                ui,
                                Vec2::new(seg_w, 36.0),
                                lbl,
                                None,
                                active,
                                Some(ACCENT_CYAN),
                            )
                            .clicked()
                            {
                                state.difficulty = diff;
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                well_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            "Motor: Interfaz Rust (egui/eframe + wgpu 3D) + Lua 5.4 + NativeActivity",
                        )
                        .strong()
                        .size(12.5)
                        .color(ACCENT_CYAN),
                    );
                    ui.label(
                        RichText::new(format!("GPU 3D activa: {gpu_info} • Superficie: {target_format:?}"))
                            .size(12.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(
                            "Soporte nativo ABI: 64-bit (arm64-v8a / x86_64) y 32-bit (armeabi-v7a).",
                        )
                        .size(11.5)
                        .color(TEXT_MUTED),
                    );
                    let cache_stats = crate::cache::inspect_cache_stats();
                    ui.label(
                        RichText::new(format!(
                            "Caché nativa ({}): Total {}  |  Mapa 3D: {}  |  Shaders wgpu: {}  |  Lua 5.4: {}  |  Historial: {}",
                            crate::cache::cache_dir().display(),
                            format_cache_bytes(cache_stats.total_bytes),
                            format_cache_bytes(cache_stats.map_bytes),
                            format_cache_bytes(cache_stats.shader_bytes),
                            format_cache_bytes(cache_stats.lua_bytes),
                            format_cache_bytes(cache_stats.history_bytes),
                        ))
                        .size(11.0)
                        .color(TEXT_MUTED),
                    );
                });

                ui.add_space(12.0);

                ui.horizontal_wrapped(|ui| {
                    if state.player_country_id.is_some()
                        && game_button(
                            ui,
                            Vec2::new(178.0, 42.0),
                            "Volver al Mapa",
                            Some(GameIcon::Map),
                            true,
                            Some(ACCENT_EMERALD),
                        )
                        .clicked()
                    {
                        action = SettingsAction::ReturnToMap;
                    }
                    if game_button(
                        ui,
                        Vec2::new(196.0, 42.0),
                        "Elegir Otra Nación",
                        Some(GameIcon::Crown),
                        false,
                        Some(ACCENT_GOLD),
                    )
                    .clicked()
                    {
                        action = SettingsAction::RestartSelectCountry;
                    }
                    if game_button(
                        ui,
                        Vec2::new(186.0, 42.0),
                        "Menú Principal",
                        Some(GameIcon::Settings),
                        false,
                        None,
                    )
                    .clicked()
                    {
                        action = SettingsAction::ReturnToMainMenu;
                    }
                    if game_button(
                        ui,
                        Vec2::new(160.0, 42.0),
                        "Limpiar Caché",
                        None,
                        false,
                        Some(ACCENT_CYAN),
                    )
                    .clicked()
                    {
                        let _ = crate::cache::clear_all_cache();
                    }
                });
            });
        });

    action
}

fn format_cache_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.2} MB", (bytes as f64) / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", (bytes as f64) / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}
